import { MAX_DOSSIER_CHARS_PER_PAGE, MAX_DOSSIER_PAGES } from "./constants";
import { quoteText } from "./chunking";
import type {
  AttentionEvent,
  DossierPage,
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
  const originChunks = chunks
    .filter((chunk) => chunk.url === session.originUrl)
    .sort((a, b) => a.index - b.index);

  if (originChunks.length === 0) return [];

  const originEvents = events
    .filter((event) => event.url === session.originUrl)
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

  const visits = Object.values(store.visits)
    .filter((visit) => visit.sessionId === sessionId)
    .sort((a, b) => a.startedAt - b.startedAt);
  const events = Object.values(store.events).filter((event) => event.sessionId === sessionId);
  const chunks = Object.values(store.chunks).filter((chunk) => chunk.sessionId === sessionId);
  const edges = Object.values(store.edges).filter((edge) => edge.sessionId === sessionId);

  const pages = visits
    .map((visit) => toDossierPage(visit, chunks.filter((chunk) => chunk.visitId === visit.id), events))
    .sort((a, b) => b.attentionScore - a.attentionScore)
    .slice(0, MAX_DOSSIER_PAGES);

  return {
    session: {
      id: session.id,
      startedAt: session.startedAt,
      stoppedAt: session.stoppedAt,
      originUrl: session.originUrl,
      originTitle: session.originTitle
    },
    pages,
    navigation: edges
      .sort((a, b) => a.createdAt - b.createdAt)
      .map((edge: NavigationEdge) => ({
        fromUrl: edge.fromUrl,
        toUrl: edge.toUrl,
        openedBy: edge.openedBy,
        transitionType: edge.transitionType,
        createdAt: edge.createdAt
      })),
    candidateResumeTargets: buildCandidateTargets(session, chunks, events),
    evidence: buildEvidenceLines(session, visits, events, chunks)
  };
}

export function buildEvidenceLines(
  session: ResearchSession,
  visits: PageVisit[],
  events: AttentionEvent[],
  chunks: PageChunk[]
): string[] {
  const branchTitles = visits
    .filter((visit) => visit.url !== session.originUrl)
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
    `Origin: ${session.originTitle || session.originUrl || "unknown"}`,
    branchTitles.length > 0 ? `Branch pages: ${branchTitles.join(" -> ")}` : "No branch pages captured yet.",
    ...selected,
    ...strongest
  ].slice(0, 10);
}

export function buildHeuristicResumeCard(store: ResumeStore, sessionId: string, note = "Local heuristic used."): ResumeCard {
  const session = store.sessions[sessionId];
  if (!session) throw new Error("No session found");

  const dossier = buildResumeDossier(store, sessionId);
  const target =
    dossier.candidateResumeTargets[0] ??
    ({
      url: session.originUrl ?? "",
      title: session.originTitle,
      textQuote: "Return to the beginning of the origin page.",
      scrollY: 0,
      reason: "No stable attention anchor was captured, so the origin page start is safest."
    } satisfies ResumeTarget);

  const branchPages = dossier.pages.filter((page) => page.url !== session.originUrl);
  const branchPhrase =
    branchPages.length > 0
      ? `You opened ${branchPages.length} supporting page${branchPages.length === 1 ? "" : "s"}: ${branchPages
          .slice(0, 3)
          .map((page) => page.title || new URL(page.url).hostname)
          .join(", ")}.`
      : "You stayed mostly on the starting page.";

  return {
    id: `card_${Date.now()}`,
    sessionId,
    createdAt: Date.now(),
    originalIntent: `You were trying to make sense of ${session.originTitle || session.originUrl || "the starting page"}.`,
    journeySummary: branchPhrase,
    newKnowledge:
      branchPages.length > 0
        ? "The side pages look like clarification stops. Fold that context back into the original article instead of restarting from the top."
        : "No major detour was captured, so the best move is to continue from the strongest reading anchor.",
    summary: `Continue from: "${target.textQuote}"`,
    confidence: target.chunkId ? 0.68 : 0.42,
    evidence: [...dossier.evidence, note].slice(0, 10),
    resumeTarget: target
  };
}
