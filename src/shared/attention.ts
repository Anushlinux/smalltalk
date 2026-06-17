import type { AttentionEvent, PageChunk } from "./types";
import { normalizeWhitespace, scoreTextMatch } from "./chunking";

const WEIGHTS: Record<AttentionEvent["kind"], number> = {
  snapshot: 0.25,
  scroll: 0.6,
  selection: 3.2,
  cursor_dwell: 1.1,
  link_click: 1.8,
  visibility: 0.5
};

export function eventWeight(event: AttentionEvent): number {
  const base = WEIGHTS[event.kind] ?? 0.5;
  if (event.kind === "selection" && event.selectedText) {
    return base + Math.min(2, normalizeWhitespace(event.selectedText).length / 160);
  }
  if (event.kind === "scroll" && typeof event.value === "number") {
    return base + Math.min(0.8, event.value);
  }
  return base;
}

export function matchEventToChunk(event: Pick<AttentionEvent, "textQuote" | "scrollY">, chunks: PageChunk[]): PageChunk | undefined {
  if (event.textQuote) {
    const scored = chunks
      .map((chunk) => ({ chunk, score: scoreTextMatch(chunk.text, event.textQuote ?? "") }))
      .sort((a, b) => b.score - a.score);
    if (scored[0]?.score >= 0.45) return scored[0].chunk;
  }

  if (typeof event.scrollY === "number") {
    return chunks
      .slice()
      .sort((a, b) => Math.abs(a.scrollY - event.scrollY!) - Math.abs(b.scrollY - event.scrollY!))[0];
  }

  return undefined;
}

export function applyAttentionScores(chunks: PageChunk[], events: AttentionEvent[]): PageChunk[] {
  const scores = new Map(chunks.map((chunk) => [chunk.id, chunk.attentionScore]));

  for (const event of events) {
    const candidates = chunks.filter((chunk) => chunk.url === event.url);
    const chunk = event.chunkId ? chunks.find((item) => item.id === event.chunkId) : matchEventToChunk(event, candidates);
    if (!chunk) continue;
    scores.set(chunk.id, (scores.get(chunk.id) ?? 0) + eventWeight(event));
  }

  return chunks.map((chunk) => ({
    ...chunk,
    attentionScore: Number((scores.get(chunk.id) ?? 0).toFixed(3))
  }));
}
