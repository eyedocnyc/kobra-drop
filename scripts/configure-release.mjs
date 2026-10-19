import fs from "node:fs";
const repository = process.env.KOBRA_UPDATE_REPOSITORY ?? "";
const pubkey = process.env.KOBRA_UPDATE_PUBLIC_KEY ?? "";
if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository) || !pubkey.trim())
  throw Error(
    "Set KOBRA_UPDATE_REPOSITORY=owner/repository and KOBRA_UPDATE_PUBLIC_KEY to the Tauri public key",
  );
const tag = process.env.GITHUB_REF_NAME;
const config = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
if (tag && tag !== `v${pkg.version}`)
  throw Error("Tag must match package.json and tauri.conf.json version");
if (config.version !== pkg.version) throw Error("App versions disagree");
if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(pkg.version))
  throw Error("Only stable release versions are supported");
const rustVersion = fs
  .readFileSync("Cargo.toml", "utf8")
  .match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (rustVersion !== pkg.version)
  throw Error("Cargo workspace and app versions disagree");
if (
  process.env.GITHUB_REPOSITORY &&
  process.env.GITHUB_REPOSITORY !== repository
)
  throw Error("The release repository must be the current workflow repository");
fs.mkdirSync(".local", { recursive: true });
fs.writeFileSync(
  ".local/release-config.json",
  JSON.stringify(
    {
      bundle: { createUpdaterArtifacts: true },
      plugins: {
        updater: {
          pubkey: pubkey.trim(),
          requireSignedVersion: true,
          endpoints: [
            `https://github.com/${repository}/releases/latest/download/latest.json`,
          ],
          windows: { installMode: "passive" },
        },
      },
    },
    null,
    2,
  ),
);
