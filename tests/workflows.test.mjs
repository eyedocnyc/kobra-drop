import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const workflows = ["ci.yml", "release.yml"].map((name) => [
  name,
  fs.readFileSync(`.github/workflows/${name}`, "utf8"),
]);

test("workflow actions are pinned to full commit SHAs", () => {
  for (const [name, text] of workflows)
    for (const [, ref] of text.matchAll(/uses:\s*\S+@(\S+)/g))
      assert.match(ref, /^[0-9a-f]{40}$/, `${name}: ${ref}`);
});

test("checkout never persists the job token", () => {
  for (const [name, text] of workflows) {
    const checkouts = text.split(/\n(?=\s*- )/).filter((s) => s.includes("actions/checkout"));
    for (const step of checkouts)
      assert.match(step, /persist-credentials:\s*false/, name);
  }
});

test("updater signing secrets are scoped to the build step only", () => {
  const release = workflows.find(([name]) => name === "release.yml")[1];
  const lines = release.split(/\r?\n/);
  const secretLines = lines
    .map((line, i) => [line, i])
    .filter(([line]) => line.includes("secrets.TAURI_SIGNING"));
  assert.equal(secretLines.length, 2);
  for (const [line, i] of secretLines) {
    // Step-level env is indented beneath a "- name:" step; job-level env is not.
    assert.ok(line.match(/^ */)[0].length >= 10, `job-level secret: ${line.trim()}`);
    const step = lines.slice(0, i).reverse().find((l) => /^\s*- /.test(l));
    assert.match(step, /tauri build|Build signed installer/, line.trim());
  }
});
