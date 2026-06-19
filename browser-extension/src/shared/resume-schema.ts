export function resumeCardSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: [
      "originalIntent",
      "journeySummary",
      "newKnowledge",
      "summary",
      "confidence",
      "evidence",
      "branchFindings",
      "suggestedNextMessage",
      "instrumentationWarnings",
      "resumeTarget"
    ],
    properties: {
      originalIntent: { type: "string" },
      journeySummary: { type: "string" },
      newKnowledge: { type: "string" },
      summary: { type: "string" },
      confidence: { type: "number", minimum: 0, maximum: 1 },
      evidence: {
        type: "array",
        items: { type: "string" },
        minItems: 1,
        maxItems: 8
      },
      branchFindings: {
        type: "array",
        items: { type: "string" },
        minItems: 0,
        maxItems: 6
      },
      suggestedNextMessage: { type: "string" },
      instrumentationWarnings: {
        type: "array",
        items: { type: "string" },
        minItems: 0,
        maxItems: 8
      },
      resumeTarget: {
        type: ["object", "null"],
        additionalProperties: false,
        required: ["url", "title", "chunkId", "heading", "textQuote", "selector", "scrollY", "wordOffset", "reason"],
        properties: {
          url: { type: "string" },
          title: { type: ["string", "null"] },
          chunkId: { type: ["string", "null"] },
          heading: { type: ["string", "null"] },
          textQuote: { type: "string" },
          selector: { type: ["string", "null"] },
          scrollY: { type: ["number", "null"] },
          wordOffset: { type: ["number", "null"] },
          reason: { type: "string" }
        }
      }
    }
  };
}
