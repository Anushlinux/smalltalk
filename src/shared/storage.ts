import { ACTIVE_SESSION_KEY, MAX_STORED_CHUNKS, MAX_STORED_EVENTS, STORAGE_KEY } from "./constants";
import type { ResumeStore } from "./types";

interface LocalStorageLike {
  get(keys: string[]): Promise<Record<string, unknown>>;
  set(values: Record<string, unknown>): Promise<void>;
}

interface BrowserStorageLike {
  local: LocalStorageLike;
}

export function emptyStore(): ResumeStore {
  return {
    sessions: {},
    visits: {},
    events: {},
    edges: {},
    chunks: {},
    cards: {}
  };
}

export function pruneStore(store: ResumeStore): ResumeStore {
  const events = Object.values(store.events).sort((a, b) => b.timestamp - a.timestamp).slice(0, MAX_STORED_EVENTS);
  const chunks = Object.values(store.chunks).sort((a, b) => b.capturedAt - a.capturedAt).slice(0, MAX_STORED_CHUNKS);

  return {
    ...store,
    events: Object.fromEntries(events.map((event) => [event.id, event])),
    chunks: Object.fromEntries(chunks.map((chunk) => [chunk.id, chunk]))
  };
}

export async function readStore(storage: BrowserStorageLike): Promise<ResumeStore> {
  const result = await storage.local.get([STORAGE_KEY, ACTIVE_SESSION_KEY]);
  const store = (result[STORAGE_KEY] as ResumeStore | undefined) ?? emptyStore();
  return {
    ...emptyStore(),
    ...store,
    activeSessionId: (result[ACTIVE_SESSION_KEY] as string | undefined) ?? store.activeSessionId
  };
}

export async function writeStore(storage: BrowserStorageLike, store: ResumeStore): Promise<void> {
  const pruned = pruneStore(store);
  await storage.local.set({
    [STORAGE_KEY]: pruned,
    [ACTIVE_SESSION_KEY]: pruned.activeSessionId
  });
}
