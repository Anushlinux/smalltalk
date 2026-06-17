import { createServer, type ServerResponse } from "node:http";
import type { ResumeCard, ResumeDossier, ResumeTarget } from "../src/shared/types";

const port = Number(process.env.PORT ?? 8787);
const model = process.env.OPENAI_MODEL ?? "gpt-4.1-mini";

createServer(async (req, res) => {
  setCors(res);

  if (req.method === "OPTIONS") {
    res.writeHead(204);
    res.end();
    return;
  }

  if (req.method === "GET" && req.url === "/health") {
    sendJson(res, 200, { ok: true, model, hasKey: Boolean(process.env.OPENAI_API_KEY) });
    return;
  }

  if (req.method !== "POST" || req.url !== "/api/resume") {
    sendJson(res, 404, { error: "Not found" });
    return;
  }

  try {
    const dossier = (await readJson(req)) as ResumeDossier;
    validateDossier(dossier);
    const card = await inferResumeCard(dossier);
    sendJson(res, 200, card);
  } catch (error) {
    sendJson(res, 500, {
      error: error instanceof Error ? error.message : String(error)
    });
  }
}).listen(port, () => {
  console.log(`Smalltalk inference proxy listening on http://localhost:${port}`);
});

async function inferResumeCard(dossier: ResumeDossier): Promise<ResumeCard> {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) throw new Error("OPENAI_API_KEY is not set.");

  const response = await fetch("https://api.openai.com/v1/responses", {
    method: "POST",
    headers: {
      authorization: `Bearer ${apiKey}`,
      "content-type": "application/json"
    },
    body: JSON.stringify({
      model,
      input: [
        {
          role: "system",
          content:
            "You are Smalltalk, a concise research-resume engine. Infer where the user should continue on the original page based on intent, attention evidence, branch-page learning, and candidate anchors. Do not merely pick the last click. Keep the tone compact and human."
        },
        {
          role: "user",
          content: JSON.stringify(dossier)
        }
      ],
      text: {
        format: {
          type: "json_schema",
          name: "smalltalk_resume_card",
          strict: true,
          schema: resumeCardSchema()
        }
      }
    })
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`OpenAI request failed (${response.status}): ${detail.slice(0, 400)}`);
  }

  const payload = await response.json();
  const parsed = parseResponseText(payload);
  return {
    ...parsed,
    id: `card_${Date.now()}`,
    sessionId: dossier.session.id,
    createdAt: Date.now()
  };
}

function parseResponseText(payload: unknown): Omit<ResumeCard, "id" | "sessionId" | "createdAt"> {
  const maybe = payload as {
    output_text?: string;
    output?: Array<{ content?: Array<{ text?: string; type?: string }> }>;
  };
  const text =
    maybe.output_text ??
    maybe.output?.flatMap((item) => item.content ?? []).find((content) => typeof content.text === "string")?.text;
  if (!text) throw new Error("OpenAI response did not include JSON text.");
  const parsed = JSON.parse(text) as Omit<ResumeCard, "id" | "sessionId" | "createdAt"> & {
    resumeTarget: Record<string, string | number | null>;
  };
  const resumeTarget = Object.fromEntries(Object.entries(parsed.resumeTarget).filter(([, value]) => value !== null)) as unknown as ResumeTarget;
  return {
    ...parsed,
    resumeTarget
  };
}

function resumeCardSchema() {
  return {
    type: "object",
    additionalProperties: false,
    required: ["originalIntent", "journeySummary", "newKnowledge", "summary", "confidence", "evidence", "resumeTarget"],
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
      resumeTarget: {
        type: "object",
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

function validateDossier(dossier: ResumeDossier) {
  if (!dossier?.session?.id) throw new Error("Missing session.");
  if (!Array.isArray(dossier.pages)) throw new Error("Missing pages.");
}

async function readJson(req: NodeJS.ReadableStream): Promise<unknown> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  const raw = Buffer.concat(chunks).toString("utf8");
  return raw ? JSON.parse(raw) : {};
}

function sendJson(res: ServerResponse, status: number, value: unknown) {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify(value));
}

function setCors(res: { setHeader: (name: string, value: string) => void }) {
  res.setHeader("access-control-allow-origin", "*");
  res.setHeader("access-control-allow-methods", "GET,POST,OPTIONS");
  res.setHeader("access-control-allow-headers", "content-type,authorization");
}
