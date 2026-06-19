import { describe, expect, it } from "vitest";
import { buildHeuristicResumeCard, buildResumeDossier, redactSensitiveText } from "../src/shared/dossier";
import { normalizeResumeCardForDossier } from "../src/shared/resume-card";
import { resumeCardSchema } from "../src/shared/resume-schema";
import type { ResumeCard, ResumeStore } from "../src/shared/types";

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
        lastSeenAt: 12,
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
      },
      c3: {
        id: "c3",
        sessionId: "s1",
        visitId: "v2",
        url: "https://docs.example.com/detail",
        title: "Clarifying detail",
        index: 0,
        text: "The supporting concept explanation was intensely useful branch evidence, but it is not where the user should resume.",
        textQuote: "The supporting concept explanation was intensely useful branch evidence.",
        scrollY: 80,
        wordStart: 0,
        wordEnd: 16,
        attentionScore: 12,
        capturedAt: 7
      }
    },
    cards: {}
  };
}

function noOriginAnchorStore(): ResumeStore {
  const store = fixtureStore();
  delete store.chunks.c1;
  delete store.chunks.c2;
  return store;
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
  it("builds branch visits and candidate origin anchors separately", () => {
    const dossier = buildResumeDossier(fixtureStore(), "s1");
    expect(dossier.mode).toBe("returned_to_origin");
    expect(dossier.branchVisits).toHaveLength(1);
    expect(dossier.navigation[0].openedBy).toBe("clicked");
    expect(dossier.candidateOriginAnchors[0].textQuote).toContain("next paragraph");
    expect(dossier.candidateOriginAnchors.every((target) => target.url === "https://example.com/origin")).toBe(true);
  });

  it("does not turn branch evidence into a resume anchor when origin chunks are missing", () => {
    const dossier = buildResumeDossier(noOriginAnchorStore(), "s1");
    expect(dossier.mode).toBe("returned_to_origin");
    expect(dossier.branchVisits[0].chunks[0].text).toContain("intensely useful branch evidence");
    expect(dossier.candidateOriginAnchors).toHaveLength(0);
    expect(dossier.instrumentationWarnings).toContain("No origin-page chunks were captured.");
  });
});

describe("buildHeuristicResumeCard", () => {
  it("continues after the last strong origin-page attention signal", () => {
    const card = buildHeuristicResumeCard(fixtureStore(), "s1");
    expect(card.resumeTarget?.chunkId).toBe("c2");
    expect(card.summary).toContain("origin page");
    expect(card.evidence.some((line) => line.includes("Branch pages"))).toBe(true);
    expect(card.branchFindings[0]).toContain("supporting concept");
  });

  it("returns no target when the origin has no candidate anchors", () => {
    const card = buildHeuristicResumeCard(noOriginAnchorStore(), "s1");
    expect(card.resumeTarget).toBeNull();
    expect(card.instrumentationWarnings).toContain("No candidate origin anchors were available.");
  });
});

describe("normalizeResumeCardForDossier", () => {
  it("nulls a branch-page target in a returned-to-origin session", () => {
    const dossier = buildResumeDossier(fixtureStore(), "s1");
    const modelCard: ResumeCard = {
      id: "card_1",
      sessionId: "s1",
      createdAt: 20,
      originalIntent: "Understand the origin.",
      journeySummary: "A branch was visited.",
      newKnowledge: "The branch was useful.",
      summary: "Bring branch research back.",
      confidence: 0.72,
      evidence: ["Branch was highly attended."],
      branchFindings: ["The supporting concept matters."],
      suggestedNextMessage: "Use the supporting concept to continue the original task.",
      instrumentationWarnings: [],
      resumeTarget: {
        url: "https://docs.example.com/detail",
        title: "Clarifying detail",
        textQuote: "The supporting concept explanation was intensely useful branch evidence.",
        reason: "The model over-weighted branch attention."
      }
    };

    const normalized = normalizeResumeCardForDossier(modelCard, dossier);
    expect(normalized.resumeTarget).toBeNull();
    expect(normalized.instrumentationWarnings.join(" ")).toContain("branch-page resume target");
  });
});

describe("resumeCardSchema", () => {
  it("allows the proxy to return a null resume target", () => {
    const schema = resumeCardSchema();
    const resumeTarget = schema.properties.resumeTarget;
    expect(resumeTarget.type).toEqual(["object", "null"]);
  });
});
