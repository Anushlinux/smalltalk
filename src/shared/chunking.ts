import { CONTENT_MIN_CHARS } from "./constants";
import type { PageChunk } from "./types";

export interface TextBlock {
  text: string;
  heading?: string;
  selector?: string;
  scrollY?: number;
}

export type ChunkDraft = Omit<
  PageChunk,
  "id" | "sessionId" | "visitId" | "url" | "title" | "attentionScore" | "capturedAt"
>;

export function normalizeWhitespace(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

export function quoteText(value: string, maxLength = 220): string {
  const normalized = normalizeWhitespace(value);
  if (normalized.length <= maxLength) return normalized;
  return normalized.slice(0, maxLength).replace(/\s+\S*$/, "").trim();
}

export function isLikelyContentText(text: string): boolean {
  const normalized = normalizeWhitespace(text);
  if (normalized.length < CONTENT_MIN_CHARS) return false;
  const wordCount = normalized.split(/\s+/).length;
  if (wordCount < 8) return false;
  const linkishRatio = (normalized.match(/https?:\/\//g) ?? []).length / Math.max(wordCount, 1);
  return linkishRatio < 0.08;
}

export function chunkTextBlocks(blocks: TextBlock[]): ChunkDraft[] {
  let wordCursor = 0;
  return blocks
    .map((block) => ({
      ...block,
      text: normalizeWhitespace(block.text)
    }))
    .filter((block) => isLikelyContentText(block.text))
    .map((block, index) => {
      const words = block.text.split(/\s+/);
      const draft: ChunkDraft = {
        index,
        heading: block.heading ? normalizeWhitespace(block.heading).slice(0, 180) : undefined,
        text: block.text,
        textQuote: quoteText(block.text),
        selector: block.selector,
        scrollY: Math.max(0, Math.round(block.scrollY ?? 0)),
        wordStart: wordCursor,
        wordEnd: wordCursor + words.length
      };
      wordCursor += words.length;
      return draft;
    });
}

export function scoreTextMatch(haystack: string, needle: string): number {
  const a = normalizeWhitespace(haystack).toLowerCase();
  const b = normalizeWhitespace(needle).toLowerCase();
  if (!a || !b) return 0;
  if (a.includes(b)) return 1;

  const bWords = new Set(b.split(/\s+/).filter((word) => word.length > 3));
  if (bWords.size === 0) return 0;
  const aWords = new Set(a.split(/\s+/).filter((word) => word.length > 3));
  let overlap = 0;
  bWords.forEach((word) => {
    if (aWords.has(word)) overlap += 1;
  });
  return overlap / bWords.size;
}
