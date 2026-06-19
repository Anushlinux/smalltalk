import { describe, expect, it } from "vitest";
import { chunkTextBlocks, scoreTextMatch } from "../src/shared/chunking";

describe("chunkTextBlocks", () => {
  it("keeps readable article blocks and assigns stable word offsets", () => {
    const chunks = chunkTextBlocks([
      { text: "Home Pricing Docs" },
      {
        heading: "Vector search",
        text: "Vector search compares semantic representations of documents instead of matching only literal words in a query.",
        selector: "p:nth-of-type(1)",
        scrollY: 120
      },
      {
        heading: "Vector search",
        text: "A resume engine should continue after the clarification point because the user now understands the missing concept.",
        selector: "p:nth-of-type(2)",
        scrollY: 260
      }
    ]);

    expect(chunks).toHaveLength(2);
    expect(chunks[0].wordStart).toBe(0);
    expect(chunks[1].wordStart).toBe(chunks[0].wordEnd);
    expect(chunks[1].heading).toBe("Vector search");
  });
});

describe("scoreTextMatch", () => {
  it("scores exact contained quotes highest", () => {
    expect(scoreTextMatch("The user should continue from this paragraph after returning.", "continue from this paragraph")).toBe(1);
    expect(scoreTextMatch("A different paragraph about pricing.", "continue from this paragraph")).toBeLessThan(0.5);
  });
});
