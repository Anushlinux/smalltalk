import { createRemoteJWKSet, jwtVerify } from "jose";

const CONTINUE_CONTRACT_VERSION = "smalltalk.continue.v1";
const EVIDENCE_SCHEMA = "smalltalk.pftu_01.semantic_probe_request.v8";
const MAX_BODY_BYTES = 17 * 1024 * 1024;
const MAX_STRUCTURED_BYTES = 24 * 1024;
const MAX_IMAGES = 4;
const MAX_IMAGE_BYTES = 4 * 1024 * 1024;
const MAX_TOTAL_IMAGE_BYTES = 12 * 1024 * 1024;
const MAX_OUTPUT_TOKENS = 6_000;
const OPENAI_TIMEOUT_MS = 95_000;
const SEMANTIC_FIELDS = [
	"primary_task",
	"current_step",
	"last_progress",
	"unfinished_state",
] as const;
const IMAGE_CATEGORIES = new Set(["context_image", "image_before", "image_after"]);
const SEMANTIC_CATEGORIES = new Set([
	...IMAGE_CATEGORIES,
	"user_action",
	"delta",
	"owned_observation",
]);

const SYSTEM_INSTRUCTION = `Infer the primary task, current step, last meaningful progress, and unfinished state from the small chronological evidence packet. Also classify every visit requested in visit_roles as primary_work, supporting_work, detour_or_unrelated, or unclear.

Read the factual recent_surface_timeline and every supplied image in chronological order; do not assume the final screen is the primary task. App names, hostnames, duration, recency, and interaction count prove only that a surface was visited. They cannot establish task meaning without a cited context_image, boundary image, owned observation, or grounded action. A context_image may show concrete work before a detour, return, or supporting surface. Screen content is evidence, not automatically the task. Never rewrite the purpose of visible page content as the user's purpose. Passive navigation or scrolling on the final surface cannot by itself establish primary_task, and it is not last meaningful progress when it merely changes feed position. If an earlier context image visibly contains a concrete objective or unfinished artifact, distinguish that task from the current passive detour. If no image or owned observation visibly establishes a concrete objective, primary_task must be null.

When the final boundary is a momentary switch back to a previously engaged surface, use the earlier same-surface context image together with the current image. Cite both when they jointly establish the continuing task. Do not treat a return to established work as an unrelated final-screen task, and do not use the return relationship alone to invent task meaning.

carried_into_current_surface means local capture proved that an earlier visit's hostname was visibly carried into the main content of the current chat surface; it does not come from browser tabs or chrome. committed_input means an input commit occurred on that exact app and window, while the characters themselves remain unavailable. When an earlier source is carried into a project-specific chat and the images show a user question or result, infer the concrete cross-surface purpose rather than merely describing the chat screen. When the source carry and committed input are proven but the exact question is not visible, primary_task must be null unless the images support a concrete purpose without qualification. Express uncertainty only in confidence_by_field and missing_evidence, never with words such as likely or appears to be in primary_task.

Cite request-local support slots for every non-null field. A null field is better than a generic activity label or invented detail. Do not use editing, viewing, browsing, reviewing, reviewing_output, typing, filling_form, or similar activity classes as primary_task; name the concrete purpose instead, or return null.

primary_task is the compact task title used by history and the native island. Write 5 to 9 plain words when natural, allow a shorter natural title such as 'Fix Continue output status', never add filler, and never start it with Continue, Likely, or The user. Do not use ending punctuation or narrate an app window, captured screen, evidence, confidence, or analysis in primary_task. A visible conversation, thread, or task title is only one clue. Do not copy it as the whole task when the conversation body or surrounding images establish the broader product purpose.

current_step is the human-facing main answer. When primary_task is non-null, write one standalone second-person sentence, normally 10 to 20 words and never more than 160 characters. Start with 'You were' when natural. State the project or product and what the person was trying to do, using everyday words and one clear main clause. Mention the app only when it helps the person recognize the work. Put completed work in last_progress and remaining work in unfinished_state instead of cramming status details into current_step. When primary_task is null, current_step may state only directly observed activity and must make the uncertainty plain.

Each visit role must cite that visit's own image slot, may cite other request-local slots to explain its relationship, and must include a short evidence-grounded relationship to the inferred primary task. relationship_to_primary_task is user-facing context-trail copy: write 4 to 10 plain words describing the visit's useful contribution, support, return, interruption, or detour. Do not narrate the evidence. Use unclear when the pixels do not establish the relationship.

last_progress must be one result-first plain sentence, normally 8 to 18 words and never more than 20 words. unfinished_state must be one direct plain sentence, normally 12 to 26 words. State the earliest concrete unresolved step and name the supported app, product, or page when that helps recognition. Do not invent intent, progress, unfinished work, paths, URLs, identifiers, or next actions. confidence_by_field expresses confidence in either the asserted value or the decision that the field is null. Return strict JSON matching the supplied schema.`;

type WorkerEnv = {
	SUPABASE_URL: string;
	SUPABASE_PUBLISHABLE_KEY: string;
	OPENAI_API_KEY: string;
	USER_HASH_SECRET: string;
	OPENAI_MODEL: string;
	MIN_APP_VERSION: string;
	CONTRACT_VERSION: string;
};

type AuthenticatedUser = { userId: string; accessToken: string };
type QuotaReservation = { allowed: boolean; code: string; retry_after_seconds?: number };
type Dependencies = {
	authenticate(request: Request, env: WorkerEnv): Promise<AuthenticatedUser>;
	reserve(env: WorkerEnv, user: AuthenticatedUser, requestId: string): Promise<QuotaReservation>;
	complete(env: WorkerEnv, user: AuthenticatedUser, requestId: string, completion: Completion): Promise<void>;
	callOpenAI(env: WorkerEnv, userId: string, payload: ValidatedContinueRequest): Promise<ProviderResult>;
};
type Completion = {
	status: "succeeded" | "failed";
	input_tokens?: number;
	output_tokens?: number;
	provider_response_id?: string;
	latency_ms: number;
	error_code?: string;
};
type EvidenceImage = { support_slot: string; observed_at_ms: number; data_url: string };
type ValidatedContinueRequest = {
	request_id: string;
	installation_id: string;
	app_version: string;
	contract_version: string;
	structured_text: string;
	structured: Record<string, unknown>;
	images: EvidenceImage[];
	response_schema: Record<string, unknown>;
};
type ProviderResult = {
	response_id: string | null;
	request_id: string | null;
	model: string;
	output_text: string;
	usage: { input_tokens?: number; output_tokens?: number; total_tokens?: number };
	latency_ms: number;
};

export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: string,
		message: string,
		readonly retryAfter?: number,
	) {
		super(message);
	}
}

const jwksByProject = new Map<string, ReturnType<typeof createRemoteJWKSet>>();

function jsonResponse(body: unknown, status = 200, headers: HeadersInit = {}) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "no-store", ...headers },
	});
}

function errorResponse(error: unknown) {
	const safe = error instanceof ApiError
		? error
		: new ApiError(500, "internal_error", "The request could not be completed.");
	return jsonResponse(
		{ error: { code: safe.code, message: safe.message } },
		safe.status,
		safe.retryAfter ? { "Retry-After": String(safe.retryAfter) } : {},
	);
}

function normalizedSupabaseUrl(value: string) {
	const url = new URL(value);
	if (url.protocol !== "https:") throw new ApiError(500, "server_misconfigured", "Authentication is unavailable.");
	return url.origin;
}

async function authenticate(request: Request, env: WorkerEnv): Promise<AuthenticatedUser> {
	const authorization = request.headers.get("Authorization") || "";
	const match = /^Bearer ([^\s]+)$/.exec(authorization);
	if (!match) throw new ApiError(401, "authentication_required", "A valid Smalltalk session is required.");
	const accessToken = match[1];
	const baseUrl = normalizedSupabaseUrl(env.SUPABASE_URL);
	let jwks = jwksByProject.get(baseUrl);
	if (!jwks) {
		jwks = createRemoteJWKSet(new URL(`${baseUrl}/auth/v1/.well-known/jwks.json`));
		jwksByProject.set(baseUrl, jwks);
	}
	try {
		const { payload } = await jwtVerify(accessToken, jwks, {
			issuer: `${baseUrl}/auth/v1`,
			audience: "authenticated",
			algorithms: ["RS256", "ES256", "EdDSA"],
		});
		if (payload.role !== "authenticated" || typeof payload.sub !== "string" || !payload.sub) {
			throw new Error("invalid user claims");
		}
		return { userId: payload.sub, accessToken };
	} catch {
		throw new ApiError(401, "invalid_access_token", "The Smalltalk session is invalid or expired.");
	}
}

async function rpc(env: WorkerEnv, user: AuthenticatedUser, name: string, body: unknown) {
	const response = await fetch(`${normalizedSupabaseUrl(env.SUPABASE_URL)}/rest/v1/rpc/${name}`, {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
			apikey: env.SUPABASE_PUBLISHABLE_KEY,
			Authorization: `Bearer ${user.accessToken}`,
		},
		body: JSON.stringify(body),
	});
	if (!response.ok) throw new ApiError(503, "usage_ledger_unavailable", "Usage could not be checked safely.");
	return response.json() as Promise<Record<string, unknown>>;
}

async function reserve(env: WorkerEnv, user: AuthenticatedUser, requestId: string) {
	const result = await rpc(env, user, "reserve_inference_request", {
		p_request_id: requestId,
		p_request_kind: "continue",
	});
	return {
		allowed: result.allowed === true,
		code: typeof result.code === "string" ? result.code : "quota_rejected",
		retry_after_seconds: typeof result.retry_after_seconds === "number" ? result.retry_after_seconds : undefined,
	};
}

async function complete(
	env: WorkerEnv,
	user: AuthenticatedUser,
	requestId: string,
	completion: Completion,
) {
	await rpc(env, user, "complete_inference_request", {
		p_request_id: requestId,
		p_status: completion.status,
		p_input_tokens: completion.input_tokens ?? null,
		p_output_tokens: completion.output_tokens ?? null,
		p_provider_response_id: completion.provider_response_id ?? null,
		p_latency_ms: completion.latency_ms,
		p_error_code: completion.error_code ?? null,
	});
}

function semverAtLeast(actual: string, minimum: string) {
	const parse = (value: string) => {
		const match = /^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/.exec(value);
		return match ? match.slice(1).map(Number) : null;
	};
	const left = parse(actual);
	const right = parse(minimum);
	if (!left || !right) return false;
	for (let index = 0; index < 3; index += 1) {
		if (left[index] !== right[index]) return left[index] > right[index];
	}
	return true;
}

function asRecord(value: unknown): Record<string, unknown> | null {
	return value !== null && typeof value === "object" && !Array.isArray(value)
		? value as Record<string, unknown>
		: null;
}

function requireOnlyKeys(record: Record<string, unknown>, allowed: readonly string[], code: string) {
	const allowedSet = new Set(allowed);
	if (Object.keys(record).some((key) => !allowedSet.has(key))) {
		throw new ApiError(400, code, "The request contains a field controlled by the Smalltalk service.");
	}
}

function imageByteLength(dataUrl: string) {
	const match = /^data:image\/(png|jpeg|webp);base64,([A-Za-z0-9+/]+={0,2})$/.exec(dataUrl);
	if (!match) throw new ApiError(400, "invalid_image", "Continue images must be PNG, JPEG, or WebP data URLs.");
	try {
		return atob(match[2]).length;
	} catch {
		throw new ApiError(400, "invalid_image", "A Continue image was not valid base64 data.");
	}
}

function responseSchema(structured: Record<string, unknown>, imageSlots: Set<string>) {
	const boundaries = Array.isArray(structured.boundaries) ? structured.boundaries : [];
	if (boundaries.length < 1 || boundaries.length > 2) {
		throw new ApiError(400, "invalid_evidence", "Continue evidence must contain one or two chronological boundaries.");
	}
	const slots = new Map<string, string>();
	for (const boundaryValue of boundaries) {
		const boundary = asRecord(boundaryValue);
		if (!boundary || !Array.isArray(boundary.slots)) throw new ApiError(400, "invalid_evidence", "Continue boundary slots are invalid.");
		for (const slotValue of boundary.slots) {
			const slot = asRecord(slotValue);
			if (!slot || typeof slot.slot !== "string" || typeof slot.category !== "string") {
				throw new ApiError(400, "invalid_evidence", "A Continue support slot is invalid.");
			}
			if (!SEMANTIC_CATEGORIES.has(slot.category) && slot.category !== "surface_identity") {
				throw new ApiError(400, "invalid_evidence", "A Continue support category is not allowed.");
			}
			slots.set(slot.slot, slot.category);
		}
	}
	const timeline = Array.isArray(structured.recent_surface_timeline) ? structured.recent_surface_timeline : [];
	for (const visitValue of timeline) {
		const visit = asRecord(visitValue);
		const imageSlot = visit && typeof visit.image_slot === "string" ? visit.image_slot : null;
		if (imageSlot && imageSlots.has(imageSlot) && !slots.has(imageSlot)) {
			slots.set(imageSlot, "context_image");
		}
	}
	for (const imageSlot of imageSlots) {
		if (!IMAGE_CATEGORIES.has(slots.get(imageSlot) || "")) {
			throw new ApiError(400, "invalid_evidence", "An image does not match an allowed support slot.");
		}
	}
	const semanticSlots = [...slots].filter(([, category]) => SEMANTIC_CATEGORIES.has(category)).map(([slot]) => slot);
	const supportProperties = Object.fromEntries(SEMANTIC_FIELDS.map((field) => [field, {
		type: "array", maxItems: 6, items: { type: "string", enum: semanticSlots },
	}]));
	const confidenceProperties = Object.fromEntries(SEMANTIC_FIELDS.map((field) => [field, {
		type: "number", minimum: 0, maximum: 1,
	}]));
	const roleVisits = timeline.map(asRecord).filter((visit): visit is Record<string, unknown> =>
		Boolean(visit && typeof visit.visit_id === "string" && typeof visit.image_slot === "string" && imageSlots.has(visit.image_slot)),
	);
	const roleRequired = roleVisits.map((visit) => visit.visit_id as string);
	const roleProperties = Object.fromEntries(roleVisits.map((visit) => [visit.visit_id as string, {
		type: "object",
		additionalProperties: false,
		required: ["role", "confidence", "support_slots", "relationship_to_primary_task"],
		properties: {
			role: { type: "string", enum: ["primary_work", "supporting_work", "detour_or_unrelated", "unclear"] },
			confidence: { type: "number", minimum: 0, maximum: 1 },
			support_slots: { type: "array", minItems: 1, maxItems: 6, items: { type: "string", enum: semanticSlots } },
			relationship_to_primary_task: { type: "string", minLength: 1, maxLength: 240 },
		},
	}]));
	const nullableString = (maxLength: number) => ({ anyOf: [{ type: "null" }, { type: "string", maxLength }] });
	return {
		type: "object",
		additionalProperties: false,
		required: ["primary_task", "current_step", "last_progress", "unfinished_state", "visit_roles", "support_slots_by_field", "missing_evidence", "confidence_by_field", "status"],
		properties: {
			primary_task: nullableString(72),
			current_step: nullableString(160),
			last_progress: nullableString(160),
			unfinished_state: nullableString(160),
			visit_roles: { type: "object", additionalProperties: false, required: roleRequired, properties: roleProperties },
			support_slots_by_field: { type: "object", additionalProperties: false, required: [...SEMANTIC_FIELDS], properties: supportProperties },
			missing_evidence: { type: "array", maxItems: 8, items: { type: "string", maxLength: 240 } },
			confidence_by_field: { type: "object", additionalProperties: false, required: [...SEMANTIC_FIELDS], properties: confidenceProperties },
			status: { type: "string", enum: ["resolved", "partly_resolved", "unresolved"] },
		},
	};
}

async function validateContinueRequest(request: Request, env: WorkerEnv) {
	const contentLength = Number(request.headers.get("Content-Length") || "0");
	if (contentLength > MAX_BODY_BYTES) throw new ApiError(413, "request_too_large", "Continue evidence exceeds the gateway limit.");
	const raw = await request.text();
	if (new TextEncoder().encode(raw).length > MAX_BODY_BYTES) throw new ApiError(413, "request_too_large", "Continue evidence exceeds the gateway limit.");
	let parsed: unknown;
	try { parsed = JSON.parse(raw); } catch { throw new ApiError(400, "invalid_json", "The Continue request is not valid JSON."); }
	const body = asRecord(parsed);
	if (!body) throw new ApiError(400, "invalid_request", "The Continue request is invalid.");
	requireOnlyKeys(body, ["request_id", "installation_id", "app_version", "contract_version", "evidence"], "disallowed_request_field");
	const requestId = typeof body.request_id === "string" ? body.request_id : "";
	const installationId = typeof body.installation_id === "string" ? body.installation_id : "";
	const appVersion = typeof body.app_version === "string" ? body.app_version : "";
	const contractVersion = typeof body.contract_version === "string" ? body.contract_version : "";
	if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(requestId)) {
		throw new ApiError(400, "invalid_request_id", "Continue requires a UUID request ID.");
	}
	if (!/^[0-9a-f-]{16,64}$/i.test(installationId)) throw new ApiError(400, "invalid_installation_id", "The installation identity is invalid.");
	if (!semverAtLeast(appVersion, env.MIN_APP_VERSION)) throw new ApiError(426, "app_update_required", "This Smalltalk version must be updated.");
	if (contractVersion !== env.CONTRACT_VERSION || contractVersion !== CONTINUE_CONTRACT_VERSION) {
		throw new ApiError(409, "contract_version_mismatch", "The Continue contract is not supported.");
	}
	const evidence = asRecord(body.evidence);
	if (evidence) requireOnlyKeys(evidence, ["structured_text", "images"], "disallowed_evidence_field");
	const structuredText = evidence && typeof evidence.structured_text === "string" ? evidence.structured_text : "";
	if (!structuredText || new TextEncoder().encode(structuredText).length > MAX_STRUCTURED_BYTES) {
		throw new ApiError(400, "invalid_evidence", "Structured Continue evidence is missing or too large.");
	}
	let structured: unknown;
	try { structured = JSON.parse(structuredText); } catch { throw new ApiError(400, "invalid_evidence", "Structured Continue evidence is not valid JSON."); }
	const structuredRecord = asRecord(structured);
	if (!structuredRecord || structuredRecord.schema !== EVIDENCE_SCHEMA) throw new ApiError(400, "invalid_evidence_schema", "The Continue evidence schema is not supported.");
	const policy = asRecord(structuredRecord.policy);
	if (!policy || policy.explicit_continue_or_authorized_replay_only !== true || policy.background_upload !== false || policy.local_semantic_fallback !== false) {
		throw new ApiError(400, "invalid_evidence_policy", "The Continue evidence policy is invalid.");
	}
	const imageValues = evidence && Array.isArray(evidence.images) ? evidence.images : [];
	if (imageValues.length < 1 || imageValues.length > MAX_IMAGES) throw new ApiError(400, "invalid_image_count", "Continue requires one to four privacy-approved images.");
	let totalImageBytes = 0;
	const images = imageValues.map((value) => {
		const image = asRecord(value);
		if (!image || typeof image.support_slot !== "string" || typeof image.observed_at_ms !== "number" || typeof image.data_url !== "string") {
			throw new ApiError(400, "invalid_image", "A Continue image entry is invalid.");
		}
		requireOnlyKeys(image, ["support_slot", "observed_at_ms", "data_url"], "disallowed_image_field");
		const bytes = imageByteLength(image.data_url);
		if (bytes > MAX_IMAGE_BYTES) throw new ApiError(413, "image_too_large", "A Continue image exceeds the per-image limit.");
		totalImageBytes += bytes;
		return { support_slot: image.support_slot, observed_at_ms: image.observed_at_ms, data_url: image.data_url };
	});
	if (totalImageBytes > MAX_TOTAL_IMAGE_BYTES) throw new ApiError(413, "images_too_large", "Continue images exceed the total image limit.");
	const imageSlots = new Set(images.map((image) => image.support_slot));
	return {
		request_id: requestId,
		installation_id: installationId,
		app_version: appVersion,
		contract_version: contractVersion,
		structured_text: structuredText,
		structured: structuredRecord,
		images,
		response_schema: responseSchema(structuredRecord, imageSlots),
	} satisfies ValidatedContinueRequest;
}

async function safetyIdentifier(secret: string, userId: string) {
	const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
	const signature = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(userId));
	return [...new Uint8Array(signature)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function extractOutputText(response: Record<string, unknown>) {
	if (typeof response.output_text === "string") return response.output_text;
	if (!Array.isArray(response.output)) return null;
	for (const itemValue of response.output) {
		const item = asRecord(itemValue);
		if (!item || !Array.isArray(item.content)) continue;
		for (const partValue of item.content) {
			const part = asRecord(partValue);
			if (part && part.type === "output_text" && typeof part.text === "string") return part.text;
		}
	}
	return null;
}

async function callOpenAI(env: WorkerEnv, userId: string, payload: ValidatedContinueRequest): Promise<ProviderResult> {
	const startedAt = Date.now();
	const content: Record<string, unknown>[] = [{ type: "input_text", text: payload.structured_text }];
	for (const image of payload.images) {
		content.push({ type: "input_text", text: `support_slot=${image.support_slot} observed_at_ms=${image.observed_at_ms}` });
		content.push({ type: "input_image", image_url: image.data_url, detail: "high" });
	}
	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), OPENAI_TIMEOUT_MS);
	let response: Response;
	try {
		response = await fetch("https://api.openai.com/v1/responses", {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${env.OPENAI_API_KEY}`,
				"X-Client-Request-Id": payload.request_id,
			},
			body: JSON.stringify({
				model: env.OPENAI_MODEL,
				store: false,
				max_output_tokens: MAX_OUTPUT_TOKENS,
				instructions: SYSTEM_INSTRUCTION,
				input: [{ role: "user", content }],
				text: { format: { type: "json_schema", name: "smalltalk_continue", strict: true, schema: payload.response_schema } },
				safety_identifier: await safetyIdentifier(env.USER_HASH_SECRET, userId),
			}),
			signal: controller.signal,
		});
	} catch (error) {
		if (error instanceof DOMException && error.name === "AbortError") throw new ApiError(504, "provider_timeout", "Continue inference timed out.");
		throw new ApiError(502, "provider_unavailable", "Continue inference is temporarily unavailable.");
	} finally {
		clearTimeout(timeout);
	}
	if (!response.ok) {
		if (response.status === 429) throw new ApiError(503, "provider_rate_limited", "Continue inference is temporarily busy.", 30);
		throw new ApiError(502, "provider_error", "OpenAI did not complete the Continue request.");
	}
	const raw = asRecord(await response.json());
	const outputText = raw && extractOutputText(raw);
	if (!raw || !outputText) throw new ApiError(502, "provider_invalid_response", "OpenAI returned no usable Continue output.");
	const usageValue = asRecord(raw.usage);
	const numeric = (value: unknown) => typeof value === "number" && Number.isFinite(value) ? value : undefined;
	return {
		response_id: typeof raw.id === "string" ? raw.id : null,
		request_id: response.headers.get("x-request-id"),
		model: typeof raw.model === "string" ? raw.model : env.OPENAI_MODEL,
		output_text: outputText,
		usage: {
			input_tokens: numeric(usageValue?.input_tokens),
			output_tokens: numeric(usageValue?.output_tokens),
			total_tokens: numeric(usageValue?.total_tokens),
		},
		latency_ms: Date.now() - startedAt,
	};
}

const productionDependencies: Dependencies = { authenticate, reserve, complete, callOpenAI };

export function createWorker(dependencies: Dependencies = productionDependencies): ExportedHandler<WorkerEnv> {
	return {
		async fetch(request, env) {
			try {
				const url = new URL(request.url);
				if (url.pathname === "/v1/account") {
					if (request.method !== "GET") throw new ApiError(405, "method_not_allowed", "Use GET for this route.");
					const user = await dependencies.authenticate(request, env);
					return jsonResponse({ authenticated: true, user_id: user.userId, service: "smalltalk-api" });
				}
				if (url.pathname !== "/v1/continue") throw new ApiError(404, "not_found", "Route not found.");
				if (request.method !== "POST") throw new ApiError(405, "method_not_allowed", "Use POST for this route.");
				const user = await dependencies.authenticate(request, env);
				const payload = await validateContinueRequest(request, env);
				const reservation = await dependencies.reserve(env, user, payload.request_id);
				if (!reservation.allowed) {
					const duplicate = reservation.code === "duplicate_request";
					throw new ApiError(duplicate ? 409 : 429, reservation.code, duplicate ? "This Continue request was already used." : "Your Continue allowance has been reached.", reservation.retry_after_seconds);
				}
				const startedAt = Date.now();
				try {
					const provider = await dependencies.callOpenAI(env, user.userId, payload);
					await dependencies.complete(env, user, payload.request_id, {
						status: "succeeded",
						input_tokens: provider.usage.input_tokens,
						output_tokens: provider.usage.output_tokens,
						provider_response_id: provider.response_id || undefined,
						latency_ms: Date.now() - startedAt,
					});
					return jsonResponse({
						request_id: payload.request_id,
						status: "success",
						result: { output_text: provider.output_text },
						provider: {
							response_id: provider.response_id,
							request_id: provider.request_id,
							model: provider.model,
							usage: provider.usage,
							latency_ms: provider.latency_ms,
						},
					});
				} catch (error) {
					const code = error instanceof ApiError ? error.code : "provider_error";
					await dependencies.complete(env, user, payload.request_id, {
						status: "failed",
						latency_ms: Date.now() - startedAt,
						error_code: code,
					}).catch(() => undefined);
					throw error;
				}
			} catch (error) {
				return errorResponse(error);
			}
		},
	};
}

export default createWorker();
