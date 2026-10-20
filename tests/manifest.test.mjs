import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

test("release manifest references the exact upload filename without spaces", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "kobra-manifest-"));
  try {
    const original = "Kobra Drop_0.1.0_x64-setup.exe";
    fs.writeFileSync(path.join(dir, original), "installer fixture");
    fs.writeFileSync(path.join(dir, original + ".sig"), "signature fixture");
    const run = () => execFileSync(process.execPath, ["scripts/manifest.mjs", dir], {
      env: { ...process.env, KOBRA_UPDATE_REPOSITORY: "example/kobra-drop" },
    });
    run();
    const manifest = JSON.parse(fs.readFileSync(path.join(dir, "latest.json")));
    const platform = manifest.platforms["windows-x86_64"];
    const filename = new URL(platform.url).pathname.split("/").pop();
    const version = JSON.parse(fs.readFileSync("package.json", "utf8")).version;
    assert.equal(filename, `KobraDrop-${version}-windows-x64-setup.exe`);
    assert.equal(fs.readFileSync(path.join(dir, filename), "utf8"), "installer fixture");
    assert.equal(fs.readFileSync(path.join(dir, filename + ".sig"), "utf8"), platform.signature);
    assert.equal(fs.existsSync(path.join(dir, original)), false);
    run(); // Regeneration keeps the same valid upload names.
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
