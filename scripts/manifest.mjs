import fs from "node:fs";
import path from "node:path";
const repository = process.env.KOBRA_UPDATE_REPOSITORY;
if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository ?? ""))
  throw Error("Invalid release repository");
const version = JSON.parse(fs.readFileSync("package.json")).version;
const dir = process.argv[2] ?? "target/release/bundle/nsis";
const files = fs.readdirSync(dir).filter((f) => f.endsWith("-setup.exe"));
if (files.length !== 1) throw Error("Expected exactly one NSIS installer");
// Use a stable filename: GitHub normalizes spaces in uploaded asset names.
const original = files[0];
const file = `KobraDrop-${version}-windows-x64-setup.exe`;
if (original !== file) {
  fs.renameSync(path.join(dir, original), path.join(dir, file));
  fs.renameSync(path.join(dir, original + ".sig"), path.join(dir, file + ".sig"));
}
const signature = fs.readFileSync(path.join(dir, file + ".sig"), "utf8").trim();
if (!signature) throw Error("Missing updater signature");
fs.writeFileSync(
  path.join(dir, "latest.json"),
  JSON.stringify(
    {
      version,
      notes: `Kobra Drop ${version}`,
      pub_date: new Date().toISOString(),
      platforms: {
        "windows-x86_64": {
          signature,
          url: `https://github.com/${repository}/releases/download/v${version}/${encodeURIComponent(file)}`,
        },
      },
    },
    null,
    2,
  ),
);
