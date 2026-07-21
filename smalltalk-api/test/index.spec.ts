import { describe, expect, it, vi } from "vitest";
import { ApiError, createWorker } from "../src/index";

const env = {
	SUPABASE_URL: "https://project.supabase.co",
	SUPABASE_PUBLISHABLE_KEY: "publishable-test",
	OPENAI_API_KEY: "server-only-test",
	USER_HASH_SECRET: "hash-secret-test",
	OPENAI_MODEL: "gpt-test",
	MIN_APP_VERSION: "0.1.0",
	CONTRACT_VERSION: "smalltalk.continue.v1",
};

function structuredEvidence() {
	return JSON.stringify({
		schema: "smalltalk.pftu_01.semantic_probe_request.v8",
		policy: {
			explicit_continue_or_authorized_replay_only: true,
			background_upload: false,
			production_authority: false,
			local_semantic_fallback: false,
		},
		recent_surface_timeline: [{ visit_id: "T1_VISIT", image_slot: "B1_IMAGE_AFTER" }],
		boundaries: [{
			boundary_index: 1,
			slots: [
				{ slot: "B1_IMAGE_AFTER", category: "image_after", observed_at_ms: 1, summary: "Current screen" },
				{ slot: "B1_USER_ACTION_1", category: "user_action", observed_at_ms: 1, summary: "Committed input" },
			],
		}],
		missing_evidence: [],
	});
}

function structuredEvidenceWithEarlierContext() {
	const structured = JSON.parse(structuredEvidence()) as Record<string, unknown>;
	structured.recent_surface_timeline = [
		{ visit_id: "T1_VISIT", image_slot: "T1_CONTEXT_IMAGE" },
		{ visit_id: "T2_VISIT", image_slot: "B1_IMAGE_AFTER" },
	];
	return JSON.stringify(structured);
}

function continueBody(extra: Record<string, unknown> = {}) {
	return {
		request_id: "019f89a1-4ef5-7b88-8db4-32f19a9cd111",
		installation_id: "019f89a1-4ef5-7b88-8db4-32f19a9cd222",
		app_version: "0.1.0",
		contract_version: "smalltalk.continue.v1",
		evidence: {
			structured_text: structuredEvidence(),
			images: [{
				support_slot: "B1_IMAGE_AFTER",
				observed_at_ms: 1,
				data_url: "data:image/png;base64,aGVsbG8=",
			}],
		},
		...extra,
	};
}

function harness(options: { quotaAllowed?: boolean } = {}) {
	const complete = vi.fn(async () => undefined);
	const dependencies = {
		authenticate: vi.fn(async (request: Request) => {
			if (request.headers.get("Authorization") !== "Bearer valid-token") throw new ApiError(401, "authentication_required", "A valid Smalltalk session is required.");
			return { userId: "user-1", accessToken: "valid-token" };
		}),
		reserve: vi.fn(async () => options.quotaAllowed === false
			? { allowed: false, code: "monthly_quota_exceeded" }
			: { allowed: true, code: "reserved" }),
		complete,
		callOpenAI: vi.fn(async (_workerEnv, _userId, payload) => ({
			response_id: "resp_1",
			request_id: "req_1",
			model: "gpt-test",
			output_text: JSON.stringify({ primary_task: "Secure the Smalltalk gateway" }),
			usage: { input_tokens: 10, output_tokens: 5, total_tokens: 15 },
			latency_ms: 20,
			payload,
		})),
	};
	return { worker: createWorker(dependencies), dependencies, complete };
}

async function fetchWorker(worker: ReturnType<typeof createWorker>, request: Request) {
	return worker.fetch!(request, env, {} as ExecutionContext);
}

describe("smalltalk-api", () => {
	it("returns safe account identity after authentication", async () => {
		const { worker } = harness();
		const response = await fetchWorker(worker, new Request("https://api.smalltalk.app/v1/account", {
			headers: { Authorization: "Bearer valid-token" },
		}));
		expect(response.status).toBe(200);
		expect(await response.json()).toEqual({ authenticated: true, user_id: "user-1", service: "smalltalk-api" });
	});

	it("rejects unauthenticated requests before quota or inference", async () => {
		const { worker, dependencies } = harness();
		const response = await fetchWorker(worker, new Request("https://api.smalltalk.app/v1/continue", {
			method: "POST",
			body: JSON.stringify(continueBody()),
		}));
		expect(response.status).toBe(401);
		expect(dependencies.reserve).not.toHaveBeenCalled();
		expect(dependencies.callOpenAI).not.toHaveBeenCalled();
	});

	it("reserves quota, calls the fixed provider contract, and completes usage", async () => {
		const { worker, dependencies, complete } = harness();
		const response = await fetchWorker(worker, new Request("https://api.smalltalk.app/v1/continue", {
			method: "POST",
			headers: { Authorization: "Bearer valid-token", "Content-Type": "application/json" },
			body: JSON.stringify(continueBody()),
		}));
		expect(response.status).toBe(200);
		const result = await response.json() as Record<string, unknown>;
		expect(result.status).toBe("success");
		expect(dependencies.reserve).toHaveBeenCalledOnce();
		expect(dependencies.callOpenAI).toHaveBeenCalledOnce();
		expect(complete).toHaveBeenCalledWith(expect.anything(), expect.anything(), expect.any(String), expect.objectContaining({ status: "succeeded", input_tokens: 10, output_tokens: 5 }));
	});

	it("accepts earlier context images declared by the surface timeline", async () => {
		const { worker, dependencies } = harness();
		const body = continueBody();
		body.evidence.structured_text = structuredEvidenceWithEarlierContext();
		body.evidence.images.unshift({
			support_slot: "T1_CONTEXT_IMAGE",
			observed_at_ms: 0,
			data_url: "data:image/png;base64,aGVsbG8=",
		});
		const response = await fetchWorker(worker, new Request("https://api.smalltalk.app/v1/continue", {
			method: "POST",
			headers: { Authorization: "Bearer valid-token", "Content-Type": "application/json" },
			body: JSON.stringify(body),
		}));
		expect(response.status).toBe(200);
		expect(dependencies.callOpenAI).toHaveBeenCalledOnce();
	});

	it("rejects client-controlled OpenAI fields", async () => {
		const { worker, dependencies } = harness();
		const response = await fetchWorker(worker, new Request("https://api.smalltalk.app/v1/continue", {
			method: "POST",
			headers: { Authorization: "Bearer valid-token", "Content-Type": "application/json" },
			body: JSON.stringify(continueBody({ model: "arbitrary-model" })),
		}));
		expect(response.status).toBe(400);
		expect(dependencies.callOpenAI).not.toHaveBeenCalled();
	});

	it("enforces the Supabase usage decision before OpenAI", async () => {
		const { worker, dependencies } = harness({ quotaAllowed: false });
		const response = await fetchWorker(worker, new Request("https://api.smalltalk.app/v1/continue", {
			method: "POST",
			headers: { Authorization: "Bearer valid-token", "Content-Type": "application/json" },
			body: JSON.stringify(continueBody()),
		}));
		expect(response.status).toBe(429);
		expect(dependencies.callOpenAI).not.toHaveBeenCalled();
	});
});
