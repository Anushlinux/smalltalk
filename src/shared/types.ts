export type SessionStatus = "active" | "stopped";

export interface ResearchSession {
  id: string;
  startedAt: number;
  stoppedAt?: number;
  status: SessionStatus;
  originTabId?: number;
  originUrl?: string;
  originTitle?: string;
  originSnapshot?: OriginSnapshot;
  visitCount: number;
  eventCount: number;
  chunkCount: number;
}

export interface PageVisit {
  id: string;
  sessionId: string;
  tabId?: number;
  url: string;
  title: string;
  startedAt: number;
  lastSeenAt: number;
  sourceVisitId?: string;
  sourceUrl?: string;
  transitionType?: string;
  openedBy?: "typed" | "clicked" | "reload" | "other";
}

export type AttentionKind =
  | "snapshot"
  | "scroll"
  | "selection"
  | "cursor_dwell"
  | "link_click"
  | "visibility";

export interface AttentionEvent {
  id: string;
  sessionId: string;
  visitId?: string;
  tabId?: number;
  url: string;
  title?: string;
  kind: AttentionKind;
  timestamp: number;
  chunkId?: string;
  textQuote?: string;
  selectedText?: string;
  targetHref?: string;
  targetText?: string;
  scrollY?: number;
  viewportHeight?: number;
  x?: number;
  y?: number;
  value?: number;
}

export interface NavigationEdge {
  id: string;
  sessionId: string;
  fromVisitId?: string;
  fromUrl?: string;
  toVisitId?: string;
  toUrl: string;
  tabId?: number;
  createdAt: number;
  transitionType?: string;
  openedBy: "typed" | "clicked" | "reload" | "other";
}

export interface PageChunk {
  id: string;
  sessionId: string;
  visitId: string;
  url: string;
  title: string;
  index: number;
  heading?: string;
  text: string;
  textQuote: string;
  selector?: string;
  scrollY: number;
  wordStart: number;
  wordEnd: number;
  attentionScore: number;
  capturedAt: number;
}

export interface ResumeTarget {
  url: string;
  title?: string;
  chunkId?: string;
  heading?: string;
  textQuote: string;
  selector?: string;
  scrollY?: number;
  wordOffset?: number;
  reason: string;
}

export interface ResumeCard {
  id: string;
  sessionId: string;
  createdAt: number;
  originalIntent: string;
  journeySummary: string;
  newKnowledge: string;
  summary: string;
  confidence: number;
  evidence: string[];
  branchFindings: string[];
  suggestedNextMessage: string;
  instrumentationWarnings: string[];
  resumeTarget: ResumeTarget | null;
}

export type AppType = "chatgpt" | "docs" | "email" | "notion" | "github" | "other";

export interface CapturedMessage {
  role: "user" | "assistant" | "unknown";
  text: string;
  selector?: string;
}

export interface OriginSnapshot {
  url: string;
  title: string;
  appType: AppType;
  visibleText: string;
  activeMessage?: CapturedMessage;
  selectedText?: string;
  centerText?: string;
  scrollY?: number;
  capturedAt: number;
}

export interface PageSnapshot {
  url: string;
  title: string;
  appType: AppType;
  visibleText: string;
  activeMessage?: CapturedMessage;
  selectedText?: string;
  centerText?: string;
  chunks: Array<Omit<PageChunk, "id" | "sessionId" | "visitId" | "attentionScore" | "capturedAt">>;
  capturedAt: number;
  scrollY: number;
}

export interface ResumeStore {
  sessions: Record<string, ResearchSession>;
  visits: Record<string, PageVisit>;
  events: Record<string, AttentionEvent>;
  edges: Record<string, NavigationEdge>;
  chunks: Record<string, PageChunk>;
  cards: Record<string, ResumeCard>;
  activeSessionId?: string;
}

export interface SessionState {
  activeSession?: ResearchSession;
  latestCard?: ResumeCard;
  savedSession?: ResearchSession;
  savedCard?: ResumeCard;
  visitCount: number;
  eventCount: number;
  chunkCount: number;
  proxyReachable: boolean;
  proxyHasKey: boolean;
  proxyModel?: string;
  proxyError?: string;
}

export interface DossierPage {
  visitId: string;
  url: string;
  title: string;
  openedBy?: PageVisit["openedBy"];
  sourceUrl?: string;
  attentionScore: number;
  selectedText: string[];
  chunks: Array<{
    chunkId: string;
    index: number;
    heading?: string;
    text: string;
    textQuote: string;
    attentionScore: number;
    selector?: string;
    scrollY: number;
    wordStart: number;
  }>;
}

export interface DepartureEvent {
  url: string;
  title?: string;
  targetHref?: string;
  targetText?: string;
  textQuote?: string;
  timestamp: number;
}

export interface ReturnEvent {
  type: "returned_to_origin";
  url: string;
  title?: string;
  timestamp: number;
  sourceUrl?: string;
}

export interface ResumeDossier {
  mode: "origin_only" | "away_from_origin" | "returned_to_origin";
  session: Pick<ResearchSession, "id" | "startedAt" | "stoppedAt" | "originUrl" | "originTitle">;
  origin: OriginSnapshot;
  departure?: DepartureEvent;
  returnEvent?: ReturnEvent;
  branchVisits: DossierPage[];
  navigation: Array<Pick<NavigationEdge, "fromUrl" | "toUrl" | "openedBy" | "transitionType" | "createdAt">>;
  candidateOriginAnchors: ResumeTarget[];
  instrumentationWarnings: string[];
  evidence: string[];
}

export type ExtensionMessage =
  | { type: "START_SESSION"; tabId?: number }
  | { type: "STOP_SESSION" }
  | { type: "SET_CAPTURE_ENABLED"; enabled: boolean }
  | { type: "GET_SESSION_STATE" }
  | { type: "CAPTURE_PAGE_SNAPSHOT"; sessionId?: string; visitId?: string }
  | { type: "PAGE_SNAPSHOT_CAPTURED"; snapshot: PageSnapshot }
  | { type: "RECORD_ATTENTION_EVENT"; event: Omit<AttentionEvent, "id" | "sessionId" | "visitId" | "tabId"> }
  | { type: "RECORD_LINK_CLICK"; event: Omit<AttentionEvent, "id" | "sessionId" | "visitId" | "tabId" | "kind"> & { kind?: "link_click" } }
  | { type: "ANALYZE_RESUME" }
  | { type: "OPEN_RESUME_TARGET"; target: ResumeTarget }
  | { type: "APPLY_RESUME_HIGHLIGHT"; target: ResumeTarget };

export type ExtensionResponse =
  | { ok: true; state?: SessionState; session?: ResearchSession; card?: ResumeCard; snapshot?: PageSnapshot }
  | { ok: false; error: string };
