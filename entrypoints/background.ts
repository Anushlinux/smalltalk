import { browser } from "wxt/browser";
import { PROXY_ORIGIN, RESUME_ENDPOINT } from "../src/shared/constants";
import { applyAttentionScores, matchEventToChunk } from "../src/shared/attention";
import { buildResumeDossier } from "../src/shared/dossier";
import { readStore, writeStore } from "../src/shared/storage";
import type {
  AttentionEvent,
  ExtensionMessage,
  ExtensionResponse,
  NavigationEdge,
  PageChunk,
  PageSnapshot,
  PageVisit,
  ResearchSession,
  ResumeCard,
  ResumeTarget,
  ResumeStore,
  SessionState
} from "../src/shared/types";

interface PendingClick {
  fromUrl: string;
  fromVisitId?: string;
  targetHref?: string;
  timestamp: number;
}

const pendingClicksByTab = new Map<number, PendingClick>();
type NavigationDetails = Pick<chrome.webNavigation.WebNavigationTransitionCallbackDetails, "frameId" | "tabId" | "url"> & {
  transitionType?: string;
};
interface ProxyHealth {
  ok: boolean;
  model?: string;
  hasKey?: boolean;
  error?: string;
}

export default defineBackground(() => {
  browser.runtime.onMessage.addListener((message: unknown, sender: unknown): Promise<ExtensionResponse> => {
    return handleMessage(message as ExtensionMessage, sender as chrome.runtime.MessageSender).catch((error) => ({
      ok: false,
      error: error instanceof Error ? error.message : String(error)
    }));
  });

  browser.webNavigation.onCommitted.addListener((details) => {
    void recordNavigation(details as NavigationDetails);
  });

  browser.webNavigation.onHistoryStateUpdated.addListener((details) => {
    void recordNavigation({ ...(details as NavigationDetails), transitionType: "history_state" });
  });
});

async function handleMessage(message: ExtensionMessage, sender: chrome.runtime.MessageSender): Promise<ExtensionResponse> {
  switch (message.type) {
    case "START_SESSION":
      return { ok: true, session: await startSession(message.tabId) };
    case "STOP_SESSION":
      await stopSession();
      return { ok: true, state: await getSessionState() };
    case "SET_CAPTURE_ENABLED":
      return { ok: false, error: "SET_CAPTURE_ENABLED is handled by the content script." };
    case "GET_SESSION_STATE":
      return { ok: true, state: await getSessionState() };
    case "PAGE_SNAPSHOT_CAPTURED":
      await recordSnapshot(message.snapshot, sender.tab?.id);
      return { ok: true };
    case "RECORD_ATTENTION_EVENT":
      await recordAttention(message.event, sender.tab?.id);
      return { ok: true };
    case "RECORD_LINK_CLICK":
      await recordLinkClick({ ...message.event, kind: "link_click" }, sender.tab?.id);
      return { ok: true };
    case "ANALYZE_RESUME":
      return { ok: true, card: await analyzeResume() };
    case "OPEN_RESUME_TARGET":
      await openAndHighlight(message.target);
      return { ok: true };
    case "APPLY_RESUME_HIGHLIGHT":
    case "CAPTURE_PAGE_SNAPSHOT":
      return { ok: false, error: `${message.type} is handled by the content script.` };
    default:
      return { ok: false, error: "Unknown message." };
  }
}

async function startSession(tabId?: number): Promise<ResearchSession> {
  const tabs = tabId
    ? [await browser.tabs.get(tabId)]
    : await browser.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0];
  if (!tab?.id || !tab.url) throw new Error("No active web tab found.");

  const now = Date.now();
  const session: ResearchSession = {
    id: `session_${now}`,
    startedAt: now,
    status: "active",
    originTabId: tab.id,
    originUrl: tab.url,
    originTitle: tab.title,
    visitCount: 1,
    eventCount: 0,
    chunkCount: 0
  };

  const store = await readStore(browser.storage);
  store.sessions[session.id] = session;
  store.activeSessionId = session.id;
  const visit = ensureVisit(store, session.id, tab.id, tab.url, tab.title ?? tab.url, {
    openedBy: "typed",
    transitionType: "session_start"
  });
  store.sessions[session.id].visitCount = countSessionItems(store.visits, session.id);
  await writeStore(browser.storage, store);

  await requestSnapshot(tab.id, session.id, visit.id);
  return (await readStore(browser.storage)).sessions[session.id];
}

async function stopSession(): Promise<void> {
  const store = await readStore(browser.storage);
  if (store.activeSessionId) {
    const session = store.sessions[store.activeSessionId];
    if (session) {
      session.status = "stopped";
      session.stoppedAt = Date.now();
    }
    store.activeSessionId = undefined;
    await writeStore(browser.storage, store);
  }
  const tabIds = await allWebTabIds();
  await setCaptureEnabled(tabIds, false);
}

async function getSessionState(): Promise<SessionState> {
  const store = await readStore(browser.storage);
  const activeSession = store.activeSessionId ? store.sessions[store.activeSessionId] : undefined;
  const sessionId = activeSession?.id;
  const cards = Object.values(store.cards)
    .filter((card) => Boolean(sessionId) && card.sessionId === sessionId)
    .sort((a, b) => b.createdAt - a.createdAt);
  const savedSession = latestStoppedSession(store);
  const savedCard = savedSession
    ? Object.values(store.cards)
        .filter((card) => card.sessionId === savedSession.id)
        .sort((a, b) => b.createdAt - a.createdAt)[0]
    : undefined;

  const health = await getProxyHealth();
  return {
    activeSession,
    latestCard: cards[0],
    savedSession,
    savedCard,
    visitCount: sessionId ? countSessionItems(store.visits, sessionId) : 0,
    eventCount: sessionId ? countSessionItems(store.events, sessionId) : 0,
    chunkCount: sessionId ? countSessionItems(store.chunks, sessionId) : 0,
    proxyReachable: health.ok,
    proxyHasKey: Boolean(health.hasKey),
    proxyModel: health.model,
    proxyError: health.error
  };
}

async function requestSnapshot(tabId: number, sessionId: string, visitId: string): Promise<void> {
  try {
    const response = await browser.tabs.sendMessage(tabId, {
      type: "CAPTURE_PAGE_SNAPSHOT",
      sessionId,
      visitId
    } satisfies ExtensionMessage);
    const typed = response as ExtensionResponse;
    if (typed.ok && typed.snapshot) await recordSnapshot(typed.snapshot, tabId);
  } catch {
    // Restricted URLs and not-yet-ready content scripts are expected in a prototype.
  }
}

async function setCaptureEnabled(tabIds: number[], enabled: boolean): Promise<void> {
  await Promise.all(
    tabIds.map((tabId) =>
      browser.tabs
        .sendMessage(tabId, {
          type: "SET_CAPTURE_ENABLED",
          enabled
        } satisfies ExtensionMessage)
        .catch(() => undefined)
    )
  );
}

async function allWebTabIds(): Promise<number[]> {
  const tabs = await browser.tabs.query({});
  return tabs
    .filter((tab) => typeof tab.id === "number" && isWebUrl(tab.url))
    .map((tab) => tab.id!);
}

async function getProxyHealth(): Promise<ProxyHealth> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 900);
  try {
    const response = await fetch(`${PROXY_ORIGIN}/health`, {
      cache: "no-store",
      signal: controller.signal
    });
    const payload = (await response.json().catch(() => ({}))) as Partial<ProxyHealth>;
    if (!response.ok) {
      return {
        ok: false,
        error: typeof payload.error === "string" ? payload.error : `Proxy health returned ${response.status}`
      };
    }
    return {
      ok: true,
      model: typeof payload.model === "string" ? payload.model : undefined,
      hasKey: Boolean(payload.hasKey)
    };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : "Proxy offline"
    };
  } finally {
    clearTimeout(timeout);
  }
}

async function recordSnapshot(snapshot: PageSnapshot, tabId?: number): Promise<void> {
  const store = await readStore(browser.storage);
  const sessionId = store.activeSessionId;
  if (!sessionId) return;
  const session = store.sessions[sessionId];
  if (!session) return;

  const visit = ensureVisit(store, sessionId, tabId, snapshot.url, snapshot.title);
  if (sameLogicalUrl(snapshot.url, session.originUrl)) {
    session.originSnapshot = {
      url: snapshot.url,
      title: snapshot.title,
      appType: snapshot.appType,
      visibleText: snapshot.visibleText,
      activeMessage: snapshot.activeMessage,
      selectedText: snapshot.selectedText,
      centerText: snapshot.centerText,
      scrollY: snapshot.scrollY,
      capturedAt: snapshot.capturedAt
    };
    session.originTitle = snapshot.title || session.originTitle;
  }

  const existingIds = new Set(
    Object.values(store.chunks)
      .filter((chunk) => chunk.visitId === visit.id)
      .map((chunk) => chunk.id)
  );
  for (const id of existingIds) delete store.chunks[id];

  for (const chunk of snapshot.chunks) {
    const id = `chunk_${visit.id}_${chunk.index}_${hashText(chunk.textQuote)}`;
    store.chunks[id] = {
      ...chunk,
      id,
      sessionId,
      visitId: visit.id,
      url: snapshot.url,
      title: snapshot.title,
      attentionScore: 0,
      capturedAt: snapshot.capturedAt
    };
  }

  const event = makeAttentionEvent(
    {
      kind: "snapshot",
      timestamp: snapshot.capturedAt,
      url: snapshot.url,
      title: snapshot.title,
      scrollY: snapshot.scrollY
    },
    sessionId,
    visit.id,
    tabId
  );
  store.events[event.id] = event;
  refreshSessionCounts(store, sessionId);
  await rescoreChunks(store, sessionId);
}

async function recordAttention(
  eventDraft: Omit<AttentionEvent, "id" | "sessionId" | "visitId" | "tabId">,
  tabId?: number
): Promise<void> {
  const store = await readStore(browser.storage);
  const sessionId = store.activeSessionId;
  if (!sessionId) return;
  const visit = ensureVisit(store, sessionId, tabId, eventDraft.url, eventDraft.title ?? eventDraft.url);
  const event = makeAttentionEvent(eventDraft, sessionId, visit.id, tabId);
  const visitChunks = Object.values(store.chunks).filter((chunk) => chunk.visitId === visit.id);
  const matched = matchEventToChunk(event, visitChunks);
  if (matched) event.chunkId = matched.id;
  store.events[event.id] = event;
  refreshSessionCounts(store, sessionId);
  await rescoreChunks(store, sessionId);
}

async function recordLinkClick(
  eventDraft: Omit<AttentionEvent, "id" | "sessionId" | "visitId" | "tabId">,
  tabId?: number
): Promise<void> {
  if (tabId) {
    const store = await readStore(browser.storage);
    const sessionId = store.activeSessionId;
    const visit = sessionId ? findVisitByTabOrUrl(store, sessionId, tabId, eventDraft.url) : undefined;
    pendingClicksByTab.set(tabId, {
      fromUrl: eventDraft.url,
      fromVisitId: visit?.id,
      targetHref: eventDraft.targetHref,
      timestamp: Date.now()
    });
  }
  await recordAttention({ ...eventDraft, kind: "link_click" }, tabId);
}

async function recordNavigation(details: NavigationDetails): Promise<void> {
  if (details.frameId !== 0 || !details.url.startsWith("http")) return;
  const store = await readStore(browser.storage);
  const sessionId = store.activeSessionId;
  if (!sessionId) return;

  const tab = await browser.tabs.get(details.tabId).catch(() => undefined);
  const pending = pendingClicksByTab.get(details.tabId);
  const transitionType = "transitionType" in details ? String(details.transitionType) : undefined;
  const openedBy = classifyOpen(pending, transitionType, details.url);
  const fromVisit = pending?.fromVisitId ? store.visits[pending.fromVisitId] : undefined;
  const visit = ensureVisit(store, sessionId, details.tabId, details.url, tab?.title ?? details.url, {
    openedBy,
    sourceVisitId: fromVisit?.id,
    sourceUrl: pending?.fromUrl,
    transitionType
  });

  const edge: NavigationEdge = {
    id: `edge_${Date.now()}_${hashText(`${pending?.fromUrl ?? ""}${details.url}`)}`,
    sessionId,
    fromVisitId: fromVisit?.id,
    fromUrl: pending?.fromUrl,
    toVisitId: visit.id,
    toUrl: details.url,
    tabId: details.tabId,
    createdAt: Date.now(),
    transitionType,
    openedBy
  };
  store.edges[edge.id] = edge;
  refreshSessionCounts(store, sessionId);
  await writeStore(browser.storage, store);

  if (pending && Date.now() - pending.timestamp > 8000) pendingClicksByTab.delete(details.tabId);
  await requestSnapshot(details.tabId, sessionId, visit.id);
}

async function analyzeResume(): Promise<ResumeCard> {
  const store = await readStore(browser.storage);
  const sessionId = store.activeSessionId ?? latestSessionId(store);
  if (!sessionId) throw new Error("Start a research session first.");

  await captureKnownTabs(store, sessionId);
  const freshStore = await readStore(browser.storage);
  const dossier = buildResumeDossier(freshStore, sessionId);

  const response = await fetch(RESUME_ENDPOINT, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(dossier)
  }).catch((error) => {
    throw new Error(`AI proxy is offline at ${RESUME_ENDPOINT}: ${error instanceof Error ? error.message : String(error)}`);
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok) {
    const message = typeof payload.error === "string" ? payload.error : `Proxy returned ${response.status}`;
    throw new Error(message);
  }

  const card = payload as ResumeCard;
  card.id ||= `card_${Date.now()}`;
  card.sessionId = sessionId;
  card.createdAt ||= Date.now();

  freshStore.cards[card.id] = card;
  await writeStore(browser.storage, freshStore);
  return card;
}

async function captureKnownTabs(store: ResumeStore, sessionId: string): Promise<void> {
  const tabIds = new Set(
    Object.values(store.visits)
      .filter((visit) => visit.sessionId === sessionId && typeof visit.tabId === "number")
      .map((visit) => visit.tabId!)
  );
  await Promise.all(
    Array.from(tabIds).map(async (tabId) => {
      const visit = Object.values(store.visits).find((item) => item.sessionId === sessionId && item.tabId === tabId);
      if (visit) await requestSnapshot(tabId, sessionId, visit.id);
    })
  );
}

async function openAndHighlight(target: ResumeTarget): Promise<void> {
  const tabs = await browser.tabs.query({ url: target.url });
  const tab = tabs[0] ?? (await browser.tabs.create({ url: target.url, active: true }));
  if (tab.id) {
    if (tab.windowId) await browser.windows.update(tab.windowId, { focused: true }).catch(() => undefined);
    await browser.tabs.update(tab.id, { active: true }).catch(() => undefined);
    setTimeout(() => {
      void browser.tabs
        .sendMessage(tab.id!, {
          type: "APPLY_RESUME_HIGHLIGHT",
          target
        } satisfies ExtensionMessage)
        .catch(() => undefined);
    }, 450);
  }
}

function ensureVisit(
  store: ResumeStore,
  sessionId: string,
  tabId: number | undefined,
  url: string,
  title: string,
  patch: Partial<PageVisit> = {}
): PageVisit {
  const existing = findVisitByTabOrUrl(store, sessionId, tabId, url);
  if (existing) {
    existing.lastSeenAt = Date.now();
    existing.title = title || existing.title;
    Object.assign(existing, patch);
    return existing;
  }

  const visit: PageVisit = {
    id: `visit_${Date.now()}_${hashText(url)}`,
    sessionId,
    tabId,
    url,
    title,
    startedAt: Date.now(),
    lastSeenAt: Date.now(),
    openedBy: patch.openedBy ?? "other",
    transitionType: patch.transitionType,
    sourceUrl: patch.sourceUrl,
    sourceVisitId: patch.sourceVisitId
  };
  store.visits[visit.id] = visit;
  return visit;
}

function findVisitByTabOrUrl(store: ResumeStore, sessionId: string, tabId: number | undefined, url: string): PageVisit | undefined {
  return Object.values(store.visits)
    .filter((visit) => visit.sessionId === sessionId)
    .find((visit) => (typeof tabId === "number" && visit.tabId === tabId && visit.url === url) || visit.url === url);
}

function makeAttentionEvent(
  event: Omit<AttentionEvent, "id" | "sessionId" | "visitId" | "tabId">,
  sessionId: string,
  visitId: string,
  tabId?: number
): AttentionEvent {
  return {
    ...event,
    id: `event_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    sessionId,
    visitId,
    tabId
  };
}

async function rescoreChunks(store: ResumeStore, sessionId: string): Promise<void> {
  const chunks = Object.values(store.chunks).filter((chunk) => chunk.sessionId === sessionId);
  const events = Object.values(store.events).filter((event) => event.sessionId === sessionId);
  const scored = applyAttentionScores(chunks, events);
  for (const chunk of scored) store.chunks[chunk.id] = chunk;
  refreshSessionCounts(store, sessionId);
  await writeStore(browser.storage, store);
}

function refreshSessionCounts(store: ResumeStore, sessionId: string): void {
  const session = store.sessions[sessionId];
  if (!session) return;
  session.visitCount = countSessionItems(store.visits, sessionId);
  session.eventCount = countSessionItems(store.events, sessionId);
  session.chunkCount = countSessionItems(store.chunks, sessionId);
}

function countSessionItems<T extends { sessionId: string }>(items: Record<string, T>, sessionId: string): number {
  return Object.values(items).filter((item) => item.sessionId === sessionId).length;
}

function latestSessionId(store: ResumeStore): string | undefined {
  return Object.values(store.sessions).sort((a, b) => b.startedAt - a.startedAt)[0]?.id;
}

function latestStoppedSession(store: ResumeStore): ResearchSession | undefined {
  return Object.values(store.sessions)
    .filter((session) => session.status === "stopped")
    .sort((a, b) => (b.stoppedAt ?? b.startedAt) - (a.stoppedAt ?? a.startedAt))[0];
}

function classifyOpen(pending: PendingClick | undefined, transitionType: string | undefined, url: string): NavigationEdge["openedBy"] {
  if (pending?.targetHref && urlsRoughlyMatch(pending.targetHref, url)) return "clicked";
  if (transitionType === "typed" || transitionType === "generated") return "typed";
  if (transitionType === "reload") return "reload";
  if (transitionType === "link") return "clicked";
  return "other";
}

function urlsRoughlyMatch(a: string, b: string): boolean {
  try {
    const left = new URL(a);
    const right = new URL(b);
    return left.origin === right.origin && left.pathname === right.pathname;
  } catch {
    return a === b;
  }
}

function sameLogicalUrl(left: string | undefined, right: string | undefined): boolean {
  if (!left || !right) return false;
  try {
    const a = new URL(left);
    const b = new URL(right);
    return a.origin === b.origin && a.pathname === b.pathname && a.search === b.search;
  } catch {
    return left === right;
  }
}

function isWebUrl(url: string | undefined): boolean {
  return Boolean(url?.startsWith("http://") || url?.startsWith("https://"));
}

function hashText(value: string): string {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash << 5) - hash + value.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash).toString(36);
}
