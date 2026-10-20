import test from "node:test";
import assert from "node:assert/strict";
import { newerStable, canInstall } from "../src/update-policy.mjs";
test("stable versions advance, with no downgrades or prereleases", () => {
  for (const [a, b, result] of [
    ["0.1.0", "0.2.0", true],
    ["0.9.0", "0.10.0", true],
    ["1.0.0", "1.0.0", false],
    ["2.0.0", "1.9.9", false],
    ["1.0.0", "2.0.0-beta.1", false],
    ["1.0.0", "bogus", false],
  ])
    assert.equal(newerStable(a, b), result);
});
test("conversion, pending selection, file dialog, shown result and install all defer installation", () => {
  const idle = {
    converting: false,
    pending: false,
    selecting: false,
    installing: false,
    // A just-converted result must stay on screen until the user opens or
    // dismisses it; installing exits the app.
    resultShown: false,
  };
  assert.equal(canInstall(idle), true);
  for (const key of Object.keys(idle))
    assert.equal(canInstall({ ...idle, [key]: true }), false);
});
