# Signing and GitHub releases

The public source and release repository is https://github.com/eyedocnyc/kobra-drop. Production updater signing is configured in GitHub Actions secrets; the public key is stored in the repository variable. Releases remain drafts until manually published. Development builds never contact placeholder URLs. The first Windows install is manual; later configured builds can update themselves.

## One-time setup for the parent/maintainer

1. Choose a public GitHub repository and push this source. Set the Actions repository variable `KOBRA_UPDATE_REPOSITORY` to `owner/repository`. Use the same repository as the workflow: its short-lived `GITHUB_TOKEN` is not authorized for a different repository.
2. Generate a **production** updater key on a trusted computer using `npm run tauri signer generate -- -w <secure-path>/kobra-drop.key`. Use a strong password. Never reuse the development test key. Do not place a production private key in source control.
3. Back up the private key and its password separately, encrypted, in at least two recoverable locations. Test restoring the backup before the first release. Losing the key/password prevents existing installs from trusting future updates. The public key alone cannot recreate it.
4. Put the **contents** of the private key into the Actions secret `TAURI_SIGNING_PRIVATE_KEY`, and the password into `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Put the complete base64 public key file contents into repository variable `KOBRA_UPDATE_PUBLIC_KEY`. Only that public verification key is shipped in the app.
5. Run the validation workflow. Before distributing the first release, import representative converted files into Anycubic Slicer Next and complete the manual checks in `VALIDATION.md`.

Updater signatures authenticate downloaded update artifacts. **Windows Authenticode signing is separate and optional**; it improves Windows publisher identification/SmartScreen behavior and requires a Windows code-signing certificate/service. The included workflow does not claim or configure Authenticode signing.

## Publish a version

Keep the version in `Cargo.toml`, `package.json`, `package-lock.json`, and `src-tauri/tauri.conf.json` aligned. Update the lockfiles through the package managers, run the tests, then push a matching stable tag such as `v0.1.0`.

The release workflow tests the conversion core, updater policy and frontend, checks the Windows backend, builds a current-user NSIS installer, and uses the official Tauri signer to produce `-setup.exe.sig`. `scripts/manifest.mjs` creates `latest.json` with the signed installer URL under public HTTPS GitHub Releases downloads. The `windows-x86_64` platform is targeted. Signing secrets are exposed only to the single build step that runs the Tauri bundler, not to dependency installation or other steps. Checkout does not persist the job token, and workflow actions are pinned to full commit SHAs. There is no GitHub credential in the app.

The workflow creates a **draft** release, uploads the installer, signature, and manifest, and does not publish it. Review artifacts and perform installation/import/update tests, then publish the draft manually. Non-tag workflow dispatch runs validation only. Published GitHub releases marked prerelease are not selected by `/releases/latest`; the app independently rejects prerelease/non-increasing versions.

For subsequent releases, retain the same production signing key, identifier, and per-user install mode. Test an update from the previous installed version before publishing. Do not delete or alter users' preferences or converted projects. Preferences live in Tauri's per-user app config directory; output projects remain beside their originals.

## Updater design

Startup schedules a check asynchronously, with an 8-second check timeout. Download has a separate 30-minute timeout; Tauri applies it to the whole request, and the NSIS update includes the offline WebView2 installer (about 210 MiB). Conversion remains local and usable when offline, the endpoint is unavailable, or verification fails. Automatic updates can be disabled; **Check for updates** remains available. Manual checking while automatic installation is disabled downloads a verified update and instructs the user to enable automatic updates to install it.

Only a strictly newer stable version is accepted. The official updater checks the public-key signature, and production configuration requires the version embedded in the signature's authenticated trusted comment to equal the manifest version. This prevents substituting an old signed artifact under an inflated version number. Do not disable `requireSignedVersion` or enable downgrades.

Downloading is separate from installation. Pending files, active conversions, file dialogs, and a just-converted result that has not yet been opened or replaced defer installation. An idle reservation in Rust excludes conversion or file selection while installing. On Windows Tauri launches the installer and then exits the app; passive NSIS installation requests relaunch automatically. The frontend's explicit relaunch is a fallback for platforms where install returns. A failed conversion retains its file selection and continues to defer updates. Preferences and user files are not removed by the updater.

Official references checked during implementation:

- [Tauri updater setup, signatures, static manifest, timeout, and Windows exit behavior](https://v2.tauri.app/plugin/updater/)
- [Tauri Windows installer and current-user NSIS mode](https://v2.tauri.app/distribute/windows-installer/)
- [Official updater configuration source, including requireSignedVersion](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/config.rs)
- [Official updater verification implementation](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs)

No private signing material is included in the public test vectors. Test fixtures contain only a harmless payload, public key, and its signature.
