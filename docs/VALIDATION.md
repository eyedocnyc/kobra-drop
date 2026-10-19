# Validation record

This record distinguishes automated checks from target-slicer and installer testing. Local validation was performed on 2026-10-19.

## Automated checks

Synthetic Rust fixtures exercise nested component paths, composed rotations/scales/translations, exact transformed vertex bounds, multiple plates and repeated instances, relative placement, conditional recentering, bed exclusions, height rejection, color slots, triangle color/support/seam painting, modifiers, object overrides, variable layers, native multi-material filament replacement, inherited profiles and cycles/missing parents, source script removal, custom layer instructions, and malformed/traversal/duplicate/DTD/compression-ratio archives. Adjacent output tests verify no overwrite, archive round trips, original-file equality, and report creation.

Updater tests exercise strictly newer stable selection, prerelease/downgrade rejection, no network call for unconfigured/opted-out startup, bounded check/download timeout options, offline check failure, failure during the official-download adapter, and deferral for conversion/pending-file/file-dialog/install states. A harmless payload signed with the official Tauri CLI verifies with the same Minisign library used by the updater; changed artifact bytes, invalid signature text, and tampered signed version comments fail. Only a public key/signature/payload are checked in. These are policy/adapter/crypto tests, not an installed app updating itself over HTTPS.

Results: **15 conversion/archive integration tests, one real cryptographic signature test, and six updater policy/adapter tests passed**. TypeScript checking and the Vite production frontend build passed. The locked Rust Windows backend passed an x86_64 MSVC cross-target compile check. Runtime JavaScript dependency audit reported zero vulnerabilities; this is not a comprehensive dependency or security audit.

A local Windows x64 debug executable and per-user NSIS installer were built using the official Tauri CLI and cargo-xwin in the existing WSL Ubuntu environment, with a workspace-local Rust toolchain and Microsoft SDK files. Windows lacked Visual Studio/MSVC build prerequisites. Cross-linking emitted missing Microsoft debug-symbol warnings; these concern debugging symbols, not conversion correctness. A Windows process startup smoke check passed, followed by a normal close. This does not verify every GUI interaction. The installer is in `target/x86_64-pc-windows-msvc/debug/bundle/nsis/` and includes the offline WebView2 installer; its size is substantially larger than the app itself. It is a local unsigned test build, with automatic-update endpoints unconfigured. Native Windows release builds remain the release workflow's responsibility.

An optimized Windows release candidate was subsequently built with the corrected filament metadata. The optimized executable is approximately 13.3 MiB; its NSIS installer is approximately 208.8 MiB because it includes the offline WebView2 runtime. A distribution copy, installation instructions, and SHA-256 checksum are in ignored `.local/packages/windows-0.1.0/`. This candidate remains unsigned with updates unconfigured and has not been published. Its installation/update cycle has not been claimed as tested.

Release configuration and manifest generation were also exercised locally with public test-signature data and a harmless dummy installer file; no endpoint was contacted or release created.

## Filament-switch import regression and fix

A user screenshot and the actual Anycubic import log exposed a defect that archive-only checks missed: scalar native preset metadata `filament_id` was serialized as an array. Anycubic's JSON reader handles that name as a string and aborted at it with `type must be string, but is array`. Options later in alphabetical order, including filament preset identity, temperatures, walls and infill, could remain at slicer defaults. This caused invalid `-1` temperature/cooling changes and an excessive-acceleration warning when changing filament profiles. Earlier statements about values written into the archive did not establish that the GUI had imported them.

The converter now excludes scalar `filament_id` from filament-option vectors, writes each native identifier into the project `filament_ids` vector, includes native project metadata and process compatibility, and validates reserved metadata types before output. The regression fixture includes real-style scalar filament IDs and checks the settings that the parser previously skipped.

The corrected real project was imported and re-exported by the installed **Anycubic Slicer Next executable**, using its headless `--export-3mf` action and an isolated data directory. It exited successfully and produced a new 3MF. The re-export retained `Anycubic PLA @Anycubic Kobra X 0.4 nozzle`, native ID `GFPLA`, nozzle temperature 205°C, cooling layer time 8 seconds, **one wall, 5% infill**, inner-wall acceleration 4000, and the Kobra X machine. The slicer normalized first-layer temperature to its nozzle-specific native value of 220°C. No active GUI session, personal profile, or original model was modified.

The user subsequently confirmed that the corrected project successfully switched to the silk PLA profile in the Anycubic GUI and sliced successfully. These are user-reported manual results for this fixture, separate from the automated and headless checks. Detailed Preview inspection, physical printing, laptop installation, and installed N → N+1 update testing are not established by this confirmation.

## Real local fixture and NozzleBuddy comparison

The downloaded dragon-egg source and its NozzleBuddy conversion were compared locally using ignored fixture storage. Neither the models, personal profiles, nor personal paths are included in the public repository. Kobra Drop successfully wrote and reopened a complete converted project and report using installed native profiles. Exact transformed geometry was **224,406 vertices, 448,764 triangles, 47.983 × 47.983 × 56.634 mm**, with Z minimum 0. One plate and one declared PLA color slot were detected. The source's cached first-layer bounding box is smaller than the full model and was not used as the geometry authority.

SHA-256 comparisons confirmed that Kobra Drop retained the root model, separate mesh part, variable layer-profile text, and object/plate/assembly metadata byte for byte. The fit check required no translation. The local NozzleBuddy output retained the mesh part and variable layers byte for byte but changed the root build translation from `(128.2, 127.99999, 28.3168831)` to `(130, 130, 28.3168831)` and reserialized object metadata.

| Setting | Source | Kobra Drop | Local NozzleBuddy output |
|---|---|---|---|
| Printer | Bambu P1S 0.4 | Kobra X 0.4 | Kobra X 0.4 |
| Walls | 1 | 1 | 1 |
| Infill | 5%, adaptive cubic | unchanged | unchanged |
| Layer height | 0.2 mm + variable profile | unchanged | unchanged |
| Supports enabled | no | no | no |
| Flow ratio | 0.98 / 0.985 source alternatives | **0.96 native** | 0.98 |
| Nozzle temperature | 220°C | **205°C native** | 220°C |
| Pressure advance | 0.02 | **0.036 native** | 0.02 |
| Minimum fan | 100% | 80% native | 80% |
| Elephant foot compensation | 0.15 mm | **0.15 preserved** | 0.1 mm |
| Support on build plate only | no | **no preserved** | yes |

The native flow/temperature/pressure values above were checked directly against the installed Anycubic PLA profile. They are not hardcoded in Kobra Drop and can change with installed profile versions. The NozzleBuddy comparison describes this one local artifact only; it does not establish how every version or input is converted. Both outputs still require actual filament selection. Kobra Drop reports source-only settings that were dropped and the two automatic support-setting translations; it also removes stale slice metadata and source plate bounding-box JSON.

## Manual checks required before distribution

- Install on a Windows standard-user account; verify per-user installation and app launch without developer tools. Verify WebView2 offline installation when necessary.
- Drop a project, choose a project, and launch through Windows Open with. Verify the saved adjacent project and report. Test an unwritable input directory and an invalid archive.
- Import the result **as a project** into Anycubic Slicer Next. Confirm the Kobra X 0.4 mm machine, plate assignments, scale, assemblies, modifiers, color painting, support painting, and variable layer heights. Select the actual filament profile for every slot; slice and inspect Preview. No successful import/slice should be inferred from archive validation alone.
- Test the actual signed N → N+1 installer update using a private staging/review release and the production public key. Verify relaunch, preserved preferences/files, corrupted signatures, offline startup, and deferral with a selected file and during conversion. A separate staging repository/key is preferable to publishing test artifacts as stable production releases.

No production release has been published. No physical print has been certified by these automated checks.
