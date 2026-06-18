import { createServer, type ServerResponse } from "node:http";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ResumeCard, ResumeDossier, ResumeTarget } from "../src/shared/types";
import { normalizeResumeCardForDossier } from "../src/shared/resume-card";
import { resumeCardSchema } from "../src/shared/resume-schema";

loadDotEnv();

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
    log(`GET /health hasKey=${Boolean(process.env.OPENAI_API_KEY)} model=${model}`);
    sendJson(res, 200, { ok: true, model, hasKey: Boolean(process.env.OPENAI_API_KEY) });
    return;
  }

  if (req.method !== "POST" || req.url !== "/api/resume") {
    sendJson(res, 404, { error: "Not found" });
    return;
  }

  try {
    log("POST /api/resume");
    const dossier = (await readJson(req)) as ResumeDossier;
    validateDossier(dossier);
    const card = await inferResumeCard(dossier);
    sendJson(res, 200, card);
  } catch (error) {
    const status = error instanceof HttpError ? error.status : error instanceof SyntaxError ? 400 : 500;
    const message = error instanceof Error ? error.message : String(error);
    log(`POST /api/resume failed status=${status} error=${message}`);
    sendJson(res, status, { error: message });
  }
}).listen(port, () => {
  console.log(`Smalltalk inference proxy listening on http://localhost:${port}`);
});

async function inferResumeCard(dossier: ResumeDossier): Promise<ResumeCard> {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) throw new HttpError(503, "OPENAI_API_KEY is not set. Add it to .env or export it before running npm run proxy.");

  log(
    `Calling OpenAI model=${model} mode=${dossier.mode} branches=${dossier.branchVisits.length} candidates=${dossier.candidateOriginAnchors.length}`
  );
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
            [
              "You are Smalltalk, a return-to-work engine.",
              "The user may have left an origin page, researched in branch pages, then returned to the origin.",
              "Your job is not to choose the most-attended branch page. Your job is to resume the original task.",
              "Rules:",
              "1. If mode is returned_to_origin, resumeTarget.url must equal dossier.origin.url.",
              "2. Branch pages are evidence only. They may contribute to branchFindings, newKnowledge, and suggestedNextMessage, never to resumeTarget.",
              "3. If candidateOriginAnchors is empty, set resumeTarget to null and explain the missing instrumentation.",
              "4. suggestedNextMessage must be something useful to type, ask, or do on the origin page.",
              "5. Prefer a compact next action over a generic summary."
            ].join("\n")
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
    log(`OpenAI failed status=${response.status} detail=${detail.slice(0, 400)}`);
    throw new HttpError(502, `OpenAI request failed (${response.status}): ${detail.slice(0, 400)}`);
  }

  const payload = await response.json();
  log("OpenAI response received");
  const parsed = parseResponseText(payload);
  return normalizeResumeCardForDossier({
    ...parsed,
    id: `card_${Date.now()}`,
    sessionId: dossier.session.id,
    createdAt: Date.now()
  }, dossier);
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
    resumeTarget: Record<string, string | number | null> | null;
  };
  const resumeTarget = parsed.resumeTarget
    ? (Object.fromEntries(Object.entries(parsed.resumeTarget).filter(([, value]) => value !== null)) as unknown as ResumeTarget)
    : null;
  return {
    ...parsed,
    branchFindings: parsed.branchFindings ?? [],
    suggestedNextMessage: parsed.suggestedNextMessage ?? parsed.summary,
    instrumentationWarnings: parsed.instrumentationWarnings ?? [],
    resumeTarget
  };
}

function validateDossier(dossier: ResumeDossier) {
  if (!dossier?.session?.id) throw new HttpError(400, "Missing session.");
  if (!dossier.origin?.url && !dossier.session.originUrl) throw new HttpError(400, "Missing origin.");
  if (!Array.isArray(dossier.branchVisits)) throw new HttpError(400, "Missing branch visits.");
  if (!Array.isArray(dossier.candidateOriginAnchors)) throw new HttpError(400, "Missing candidate origin anchors.");
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

class HttpError extends Error {
  constructor(
    public readonly status: number,
    message: string
  ) {
    super(message);
  }
}

function loadDotEnv() {
  const envPath = resolve(process.cwd(), ".env");
  if (!existsSync(envPath)) return;

  const lines = readFileSync(envPath, "utf8").split(/\r?\n/);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const match = trimmed.match(/^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/);
    if (!match) continue;
    const [, key, rawValue] = match;
    if (process.env[key] != null) continue;
    process.env[key] = unquoteEnvValue(rawValue.trim());
  }
}

function unquoteEnvValue(value: string): string {
  if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
    return value.slice(1, -1);
  }
  return value;
}

function log(message: string) {
  console.log(`[smalltalk-proxy] ${new Date().toISOString()} ${message}`);
}
