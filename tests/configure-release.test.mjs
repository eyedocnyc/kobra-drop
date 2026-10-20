import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

test("release configuration binds signed versions and never allows downgrades", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "kobra-release-"));
  try {
    fs.mkdirSync(path.join(dir, "src-tauri"));
    for (const file of ["package.json", "Cargo.toml", "src-tauri/tauri.conf.json"])
      fs.copyFileSync(file, path.join(dir, file));
    execFileSync(process.execPath, [path.resolve("scripts/configure-release.mjs")], {
      cwd: dir,
      env: {
        ...process.env,
        KOBRA_UPDATE_REPOSITORY: "example/kobra-drop",
        KOBRA_UPDATE_PUBLIC_KEY: "test-public-key",
        GITHUB_REPOSITORY: "example/kobra-drop",
        GITHUB_REF_NAME: "",
      },
    });
    const config = JSON.parse(
      fs.readFileSync(path.join(dir, ".local/release-config.json"), "utf8"),
    );
    const updater = config.plugins.updater;
    assert.equal(updater.requireSignedVersion, true);
    assert.notEqual(updater.allowDowngrades, true);
    assert.equal(updater.pubkey, "test-public-key");
    assert.deepEqual(updater.endpoints, [
      "https://github.com/example/kobra-drop/releases/latest/download/latest.json",
    ]);
    assert.equal(config.bundle.createUpdaterArtifacts, true);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
