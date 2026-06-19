import { MAX_DOSSIER_CHARS_PER_PAGE, MAX_DOSSIER_PAGES } from "./constants";
import { quoteText } from "./chunking";
import type {
  AppType,
  AttentionEvent,
  DossierPage,
  OriginSnapshot,
  NavigationEdge,
  PageChunk,
  PageVisit,
  ResearchSession,
  ResumeCard,
  ResumeDossier,
  ResumeStore,
  ResumeTarget
} from "./types";

export function redactSensitiveText(value: string): string {
  return value
    .replace(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi, "[email]")
    .replace(/\b(?:\+?\d[\d -]{8,}\d)\b/g, "[phone_or_number]")
    .replace(/\b(?:sk|pk|rk|sess|key|token|secret)[-_][A-Za-z0-9_-]{16,}\b/g, "[secret]")
    .replace(/\b[A-Za-z0-9_-]{32,}\b/g, "[long_token]");
}

function pageAttention(visit: PageVisit, chunks: PageChunk[], events: AttentionEvent[]): number {
  const chunkScore = chunks.reduce((sum, chunk) => sum + chunk.attentionScore, 0);
  const eventScore = events.filter((event) => event.visitId === visit.id).length * 0.15;
  return Number((chunkScore + eventScore).toFixed(3));
}

function selectedTextForVisit(visit: PageVisit, events: AttentionEvent[]): string[] {
  return events
    .filter((event) => event.visitId === visit.id && event.kind === "selection" && event.selectedText)
    .map((event) => redactSensitiveText(event.selectedText ?? ""))
    .slice(-5);
}

function toDossierPage(visit: PageVisit, chunks: PageChunk[], events: AttentionEvent[]): DossierPage {
  let chars = 0;
  const pageChunks = chunks
    .slice()
    .sort((a, b) => b.attentionScore - a.attentionScore || a.index - b.index)
    .filter((chunk) => {
      if (chars > MAX_DOSSIER_CHARS_PER_PAGE) return false;
      chars += chunk.text.length;
      return true;
    })
    .sort((a, b) => a.index - b.index)
    .map((chunk) => ({
      chunkId: chunk.id,
      index: chunk.index,
      heading: chunk.heading,
      text: redactSensitiveText(chunk.text),
      textQuote: redactSensitiveText(chunk.textQuote),
      attentionScore: chunk.attentionScore,
      selector: chunk.selector,
      scrollY: chunk.scrollY,
      wordStart: chunk.wordStart
    }));

  return {
    visitId: visit.id,
    url: visit.url,
    title: visit.title,
    openedBy: visit.openedBy,
    sourceUrl: visit.sourceUrl,
    attentionScore: pageAttention(visit, chunks, events),
    selectedText: selectedTextForVisit(visit, events),
    chunks: pageChunks
  };
}

export function buildCandidateTargets(session: ResearchSession, chunks: PageChunk[], events: AttentionEvent[]): ResumeTarget[] {
  const originUrl = session.originSnapshot?.url ?? session.originUrl;
  const originChunks = chunks
    .filter((chunk) => sameLogicalUrl(chunk.url, originUrl))
    .sort((a, b) => a.index - b.index);

  if (originChunks.length === 0) return [];

  const originEvents = events
    .filter((event) => sameLogicalUrl(event.url, originUrl))
    .sort((a, b) => a.timestamp - b.timestamp);

  const lastStrongQuote = originEvents
    .filter((event) => event.kind === "selection" || event.kind === "cursor_dwell" || event.kind === "link_click")
    .at(-1)?.textQuote;

  const lastStrongIndex = lastStrongQuote
    ? originChunks.findIndex((chunk) => chunk.text.includes(lastStrongQuote) || chunk.textQuote.includes(lastStrongQuote))
    : -1;

  const firstUnread = originChunks[Math.min(originChunks.length - 1, Math.max(0, lastStrongIndex + 1))];
  const topAttention = originChunks.slice().sort((a, b) => b.attentionScore - a.attentionScore)[0];
  const candidates = [firstUnread, topAttention, originChunks[0]].filter(Boolean);
  const seen = new Set<string>();

  return candidates
    .filter((chunk) => {
      if (seen.has(chunk.id)) return false;
      seen.add(chunk.id);
      return true;
    })
    .map((chunk) => ({
      url: chunk.url,
      title: chunk.title,
      chunkId: chunk.id,
      heading: chunk.heading,
      textQuote: chunk.textQuote,
      selector: chunk.selector,
      scrollY: chunk.scrollY,
      wordOffset: chunk.wordStart,
      reason:
        chunk.id === firstUnread.id
          ? "This is the first origin-page chunk after the last strong attention signal."
          : "This chunk carried the strongest attention evidence on the origin page."
    }));
}

export function buildResumeDossier(store: ResumeStore, sessionId: string): ResumeDossier {
  const session = store.sessions[sessionId];
  if (!session) throw new Error("No session found");
  const originUrl = session.originSnapshot?.url ?? session.originUrl;

  const visits = Object.values(store.visits)
    .filter((visit) => visit.sessionId === sessionId)
    .sort((a, b) => a.startedAt - b.startedAt);
  const events = Object.values(store.events).filter((event) => event.sessionId === sessionId);
  const chunks = Object.values(store.chunks).filter((chunk) => chunk.sessionId === sessionId);
  const edges = Object.values(store.edges).filter((edge) => edge.sessionId === sessionId);
  const originChunks = chunks.filter((chunk) => sameLogicalUrl(chunk.url, originUrl));
  const branchVisitRecords = visits.filter((visit) => !sameLogicalUrl(visit.url, originUrl));
  const candidateOriginAnchors = buildCandidateTargets(session, chunks, events);
  const returnEvent = buildReturnEvent(originUrl, visits, edges);
  const mode = returnEvent ? "returned_to_origin" : branchVisitRecords.length > 0 ? "away_from_origin" : "origin_only";

  const branchVisits = branchVisitRecords
    .map((visit) => toDossierPage(visit, chunks.filter((chunk) => chunk.visitId === visit.id), events))
    .sort((a, b) => b.attentionScore - a.attentionScore)
    .slice(0, MAX_DOSSIER_PAGES);

  return {
    mode,
    session: {
      id: session.id,
      startedAt: session.startedAt,
      stoppedAt: session.stoppedAt,
      originUrl: session.originUrl,
      originTitle: session.originTitle
    },
    origin: buildOriginSnapshot(session, originChunks, events),
    departure: buildDepartureEvent(originUrl, events),
    returnEvent,
    branchVisits,
    navigation: edges
      .sort((a, b) => a.createdAt - b.createdAt)
      .map((edge: NavigationEdge) => ({
        fromUrl: edge.fromUrl,
        toUrl: edge.toUrl,
        openedBy: edge.openedBy,
        transitionType: edge.transitionType,
        createdAt: edge.createdAt
      })),
    candidateOriginAnchors,
    instrumentationWarnings: buildInstrumentationWarnings(session, originChunks, candidateOriginAnchors, branchVisitRecords, returnEvent),
    evidence: buildEvidenceLines(session, visits, events, chunks)
  };
}

function buildOriginSnapshot(session: ResearchSession, originChunks: PageChunk[], events: AttentionEvent[]): OriginSnapshot {
  const stored = session.originSnapshot;
  if (stored) {
    return {
      ...stored,
      visibleText: redactSensitiveText(stored.visibleText),
      activeMessage: stored.activeMessage
        ? {
            ...stored.activeMessage,
            text: redactSensitiveText(stored.activeMessage.text)
          }
        : undefined,
      selectedText: stored.selectedText ? redactSensitiveText(stored.selectedText) : undefined,
      centerText: stored.centerText ? redactSensitiveText(stored.centerText) : undefined
    };
  }

  const originUrl = session.originUrl ?? "";
  const originEvents = events
    .filter((event) => sameLogicalUrl(event.url, originUrl))
    .sort((a, b) => a.timestamp - b.timestamp);
  const activeEvent = originEvents
    .filter((event) => event.kind === "selection" || event.kind === "cursor_dwell" || event.kind === "scroll" || event.kind === "link_click")
    .at(-1);
  const visibleText = originChunks
    .sort((a, b) => a.index - b.index)
    .map((chunk) => redactSensitiveText(chunk.text))
    .join("\n")
    .slice(0, MAX_DOSSIER_CHARS_PER_PAGE);

  return {
    url: originUrl,
    title: session.originTitle ?? originUrl,
    appType: inferAppType(originUrl),
    visibleText,
    activeMessage: activeEvent?.textQuote
      ? {
          role: "unknown",
          text: redactSensitiveText(activeEvent.textQuote)
        }
      : undefined,
    selectedText: originEvents
      .filter((event) => event.kind === "selection" && event.selectedText)
      .map((event) => redactSensitiveText(event.selectedText ?? ""))
      .at(-1),
    centerText: activeEvent?.textQuote ? redactSensitiveText(activeEvent.textQuote) : undefined,
    scrollY: activeEvent?.scrollY ?? originChunks[0]?.scrollY,
    capturedAt: activeEvent?.timestamp ?? originChunks[0]?.capturedAt ?? session.startedAt
  };
}

function buildDepartureEvent(originUrl: string | undefined, events: AttentionEvent[]): ResumeDossier["departure"] {
  const link = events
    .filter((event) => event.kind === "link_click" && sameLogicalUrl(event.url, originUrl))
    .sort((a, b) => a.timestamp - b.timestamp)
    .at(-1);
  if (!link) return undefined;

  return {
    url: link.url,
    title: link.title,
    targetHref: link.targetHref,
    targetText: link.targetText ? redactSensitiveText(link.targetText) : undefined,
    textQuote: link.textQuote ? redactSensitiveText(link.textQuote) : undefined,
    timestamp: link.timestamp
  };
}

function buildReturnEvent(
  originUrl: string | undefined,
  visits: PageVisit[],
  edges: NavigationEdge[]
): ResumeDossier["returnEvent"] {
  const branchVisits = visits.filter((visit) => !sameLogicalUrl(visit.url, originUrl));
  const lastVisit = visits.slice().sort((a, b) => a.lastSeenAt - b.lastSeenAt).at(-1);
  if (branchVisits.length === 0 || !lastVisit || !sameLogicalUrl(lastVisit.url, originUrl)) return undefined;
  const returnEdge = edges
    .filter((edge) => sameLogicalUrl(edge.toUrl, originUrl))
    .sort((a, b) => b.createdAt - a.createdAt)[0];

  return {
    type: "returned_to_origin",
    url: lastVisit.url,
    title: lastVisit.title,
    timestamp: lastVisit.lastSeenAt,
    sourceUrl: returnEdge?.fromUrl
  };
}

function buildInstrumentationWarnings(
  session: ResearchSession,
  originChunks: PageChunk[],
  candidateOriginAnchors: ResumeTarget[],
  branchVisits: PageVisit[],
  returnEvent: ResumeDossier["returnEvent"]
): string[] {
  const warnings: string[] = [];
  if (!session.originUrl) warnings.push("No origin URL was captured for this session.");
  if (originChunks.length === 0) warnings.push("No origin-page chunks were captured.");
  if (candidateOriginAnchors.length === 0) warnings.push("No candidate origin anchors were available.");
  if (branchVisits.length > 0 && !returnEvent) warnings.push("The session has branch research, but no return-to-origin event was captured.");
  if (!session.originSnapshot?.visibleText && originChunks.length === 0) warnings.push("Origin visible text is missing.");
  return Array.from(new Set(warnings)).slice(0, 6);
}

export function buildEvidenceLines(
  session: ResearchSession,
  visits: PageVisit[],
  events: AttentionEvent[],
  chunks: PageChunk[]
): string[] {
  const originUrl = session.originSnapshot?.url ?? session.originUrl;
  const branchTitles = visits
    .filter((visit) => !sameLogicalUrl(visit.url, originUrl))
    .slice(-5)
    .map((visit) => visit.title || visit.url);
  const selected = events
    .filter((event) => event.kind === "selection" && event.selectedText)
    .slice(-4)
    .map((event) => `Selected: ${quoteText(redactSensitiveText(event.selectedText ?? ""), 160)}`);
  const strongest = chunks
    .slice()
    .sort((a, b) => b.attentionScore - a.attentionScore)
    .slice(0, 3)
    .map((chunk) => `Attention on ${chunk.title}: ${quoteText(redactSensitiveText(chunk.text), 160)}`);

  return [
    `Origin: ${session.originTitle || originUrl || "unknown"}`,
    branchTitles.length > 0 ? `Branch pages: ${branchTitles.join(" -> ")}` : "No branch pages captured yet.",
    ...selected,
    ...strongest
  ].slice(0, 10);
}

export function buildBranchFindings(branchVisits: DossierPage[]): string[] {
  const selected = branchVisits.flatMap((page) => page.selectedText.map((text) => quoteText(text, 180)));
  const chunks = branchVisits
    .flatMap((page) =>
      page.chunks.map((chunk) => ({
        title: page.title,
        text: chunk.text,
        score: chunk.attentionScore
      }))
    )
    .sort((a, b) => b.score - a.score)
    .map((item) => `${item.title}: ${quoteText(item.text, 180)}`);

  return Array.from(new Set([...selected, ...chunks])).slice(0, 5);
}

export function buildHeuristicResumeCard(store: ResumeStore, sessionId: string, note = "Local heuristic used."): ResumeCard {
  const session = store.sessions[sessionId];
  if (!session) throw new Error("No session found");

  const dossier = buildResumeDossier(store, sessionId);
  const target = dossier.candidateOriginAnchors[0] ?? null;
  const branchFindings = buildBranchFindings(dossier.branchVisits);
  const branchPages = dossier.branchVisits;
  const branchPhrase =
    branchPages.length > 0
      ? `You opened ${branchPages.length} supporting page${branchPages.length === 1 ? "" : "s"}: ${branchPages
          .slice(0, 3)
          .map((page) => page.title || safeHostname(page.url))
          .join(", ")}.`
      : "You stayed mostly on the starting page.";
  const suggestedNextMessage =
    branchFindings.length > 0
      ? `I checked supporting material about this. Use these points to help me continue the original task: ${branchFindings
          .slice(0, 3)
          .join(" ")}`
      : "Help me continue from the original task using the context visible in this conversation.";

  return {
    id: `card_${Date.now()}`,
    sessionId,
    createdAt: Date.now(),
    originalIntent: `You were trying to make sense of ${session.originTitle || session.originUrl || "the starting page"}.`,
    journeySummary: branchPhrase,
    newKnowledge:
      branchPages.length > 0
        ? "The side pages are branch evidence. Bring their useful points back to the original task instead of resuming on the branch page."
        : "No major detour was captured, so the best move is to continue from the strongest reading anchor.",
    summary: target
      ? `Continue on the origin page from: "${target.textQuote}"`
      : "No safe origin anchor was captured. Use the suggested next message instead.",
    confidence: target?.chunkId ? 0.68 : 0.38,
    evidence: [...dossier.evidence, note].slice(0, 10),
    branchFindings,
    suggestedNextMessage,
    instrumentationWarnings: dossier.instrumentationWarnings,
    resumeTarget: target
  };
}

function inferAppType(url: string | undefined): AppType {
  if (!url) return "other";
  try {
    const host = new URL(url).hostname;
    if (host.includes("chatgpt.com")) return "chatgpt";
    if (host.includes("github.com")) return "github";
    if (host.includes("notion.so")) return "notion";
    if (host.includes("mail.google.com")) return "email";
    if (host.includes("docs.google.com") || host.includes("developer.chrome.com")) return "docs";
  } catch {
    return "other";
  }
  return "other";
}

function safeHostname(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
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
