import assert from "node:assert/strict";
import test from "node:test";

import {
  ContinueRequestTimeoutError,
  continueRequestErrorCopy,
  isContinueRequestTimeout,
  isTransientScreenshotCaptureContention,
  withContinueRequestTimeout,
} from "../src/continueRequest.ts";

test("returns a Continue result that finishes before the deadline", async () => {
  const result = await withContinueRequestTimeout(Promise.resolve("ready"), 100);
  assert.equal(result, "ready");
});

test("rejects a Continue request that exceeds the deadline", async () => {
  const neverFinishes = new Promise(() => {});

  await assert.rejects(
    withContinueRequestTimeout(neverFinishes, 5),
    (error) => {
      assert.ok(error instanceof ContinueRequestTimeoutError);
      assert.equal(isContinueRequestTimeout(error), true);
      return true;
    },
  );
});

test("turns backend coordination timeouts into clear product copy", () => {
  assert.equal(
    continueRequestErrorCopy(
      "workload governor timed out waiting for manualcontinue",
    ),
    "Continue could not start because an earlier capture or refresh was still finishing. The previous answer is still available; please try again.",
  );
});

test("recognizes transient screenshot admission collisions", () => {
  assert.equal(
    isTransientScreenshotCaptureContention(
      "workload governor timed out waiting for screenshotcapture",
    ),
    true,
  );
  assert.equal(
    isTransientScreenshotCaptureContention(
      "workload governor timed out waiting for screenshotmemory",
    ),
    true,
  );
  assert.equal(
    isTransientScreenshotCaptureContention("screen recording permission denied"),
    false,
  );
});
