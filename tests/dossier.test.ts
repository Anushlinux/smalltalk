import { describe, expect, it } from "vitest";
import { buildHeuristicResumeCard, buildResumeDossier, redactSensitiveText } from "../src/shared/dossier";
import type { ResumeStore } from "../src/shared/types";

function fixtureStore(): ResumeStore {
  return {
    activeSessionId: "s1",
    sessions: {
      s1: {
        id: "s1",
        startedAt: 1,
        status: "active",
        originUrl: "https://example.com/origin",
        originTitle: "Original research",
        visitCount: 2,
        eventCount: 2,
        chunkCount: 2
      }
    },
    visits: {
      v1: {
        id: "v1",
        sessionId: "s1",
        url: "https://example.com/origin",
        title: "Original research",
        startedAt: 1,
        lastSeenAt: 5,
        openedBy: "typed"
      },
      v2: {
        id: "v2",
        sessionId: "s1",
        url: "https://docs.example.com/detail",
        title: "Clarifying detail",
        startedAt: 6,
        lastSeenAt: 9,
        openedBy: "clicked",
        sourceUrl: "https://example.com/origin"
      }
    },
    events: {
      e1: {
        id: "e1",
        sessionId: "s1",
        visitId: "v1",
        url: "https://example.com/origin",
        kind: "selection",
        timestamp: 3,
        selectedText: "The confusing sentence the user paused on",
        textQuote: "The confusing sentence the user paused on"
      },
      e2: {
        id: "e2",
        sessionId: "s1",
        visitId: "v2",
        url: "https://docs.example.com/detail",
        kind: "cursor_dwell",
        timestamp: 8,
        textQuote: "The supporting concept explanation"
      }
    },
    edges: {
      n1: {
        id: "n1",
        sessionId: "s1",
        fromUrl: "https://example.com/origin",
        toUrl: "https://docs.example.com/detail",
        createdAt: 6,
        openedBy: "clicked"
      }
    },
    chunks: {
      c1: {
        id: "c1",
        sessionId: "s1",
        visitId: "v1",
        url: "https://example.com/origin",
        title: "Original research",
        index: 0,
        text: "The confusing sentence the user paused on before opening a supporting tab for context.",
        textQuote: "The confusing sentence the user paused on before opening a supporting tab for context.",
        scrollY: 100,
        wordStart: 0,
        wordEnd: 13,
        attentionScore: 4,
        capturedAt: 2
      },
      c2: {
        id: "c2",
        sessionId: "s1",
        visitId: "v1",
        url: "https://example.com/origin",
        title: "Original research",
        index: 1,
        text: "The next paragraph is the right place to resume because it builds on the supporting concept.",
        textQuote: "The next paragraph is the right place to resume because it builds on the supporting concept.",
        scrollY: 240,
        wordStart: 13,
        wordEnd: 28,
        attentionScore: 1,
        capturedAt: 2
      }
    },
    cards: {}
  };
}

describe("redactSensitiveText", () => {
  it("removes obvious emails, tokens, and long numbers", () => {
    const redacted = redactSensitiveText("mail me at hello@example.com with sk-secret_abcdefghijklmnopqrstuvwxyz or +1 555 222 1111");
    expect(redacted).not.toContain("hello@example.com");
    expect(redacted).not.toContain("abcdefghijklmnopqrstuvwxyz");
    expect(redacted).toContain("[email]");
  });
});

describe("buildResumeDossier", () => {
  it("builds compact pages and candidate resume targets", () => {
    const dossier = buildResumeDossier(fixtureStore(), "s1");
    expect(dossier.pages).toHaveLength(2);
    expect(dossier.navigation[0].openedBy).toBe("clicked");
    expect(dossier.candidateResumeTargets[0].textQuote).toContain("next paragraph");
  });
});

describe("buildHeuristicResumeCard", () => {
  it("continues after the last strong origin-page attention signal", () => {
    const card = buildHeuristicResumeCard(fixtureStore(), "s1");
    expect(card.resumeTarget.chunkId).toBe("c2");
    expect(card.summary).toContain("Continue from");
    expect(card.evidence.some((line) => line.includes("Branch pages"))).toBe(true);
  });
});
