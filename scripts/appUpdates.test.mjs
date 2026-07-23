import assert from "node:assert/strict";
import test from "node:test";

import {
  appUpdateStatusCopy,
  updateProgressPercent,
} from "../src/updates/updatePresentation.ts";

test("update progress is bounded and handles unknown totals", () => {
  assert.equal(updateProgressPercent(50, 100), 50);
  assert.equal(updateProgressPercent(120, 100), 100);
  assert.equal(updateProgressPercent(-10, 100), 0);
  assert.equal(updateProgressPercent(10, null), null);
  assert.equal(updateProgressPercent(10, 0), null);
});

test("available update copy names both versions", () => {
  assert.deepEqual(
    appUpdateStatusCopy({
      phase: "available",
      currentVersion: "0.1.0",
      availableVersion: "0.1.1",
      progressPercent: null,
      errorContext: null,
    }),
    {
      title: "Smalltalk 0.1.1 is available",
      detail: "You are currently using 0.1.0.",
    },
  );
});

test("install failures do not claim that the app changed", () => {
  assert.deepEqual(
    appUpdateStatusCopy({
      phase: "error",
      currentVersion: "0.1.0",
      availableVersion: "0.1.1",
      progressPercent: null,
      errorContext: "install",
    }),
    {
      title: "Update could not be installed",
      detail: "Nothing was changed. Check your connection and try again.",
    },
  );
});
