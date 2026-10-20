# Kobra Drop

A local Windows desktop app that converts editable MakerWorld/Bambu 3MF projects for **Anycubic Slicer Next, Anycubic Kobra X, 0.4 mm nozzle**. Built with Tauri 2, TypeScript, and Rust. No Python, account, preset installation, telemetry, or model uploads. Conversion has no network dependency.

## Using the app

1. Install Anycubic Slicer Next with its Kobra X 0.4 mm profiles, then install Kobra Drop using the per-user `-setup.exe` installer.
2. Drop one editable `.3mf`, or click **Choose file**.
3. Click **Convert for Kobra X**. The original is never overwritten. The app writes an adjacent `_KobraX.3mf` and `.report.txt`, adding a number if needed.
4. Click **Open in Anycubic Slicer Next** and open as a **project**.
5. **Choose your filament, slice, and check Preview.** Select the actual spool profile for every color slot. Check supports, clearances, color changes, plate placement, and purge-tower position.

Settings lets a parent locate the slicer's profiles directory or executable if automatic discovery fails. Native profiles are read without modification. Windows **Open with → Kobra Drop** is supported; additional files are accepted when the current selection is clear.

The first version deliberately refuses unsupported material families, non-millimeter units, unknown required 3MF extensions, unsupported bed shapes/exclusions, ambiguous quality values, missing component/plate references, unsafe geometry configurations, and out-of-range layer heights. It reports dropped settings and custom commands. It never automatically scales or repairs a mesh, increases walls, enables supports, or changes fit compensation values.

## Developer setup

On Windows install Node.js 22.12+ (or a current LTS), Rust stable **1.90+**, and Visual Studio Build Tools with **Desktop development with C++** and a Windows SDK. WebView2 is required at runtime; the NSIS package includes its offline installer.

```powershell
npm ci
npm test
npm run build
cargo test -p kobra-converter --locked
npm run tauri dev
```

Build the local development installer with `npm run tauri build`. Artifacts are in `target/release/bundle/nsis/`. This build has updates disabled and contacts no placeholder endpoints. It can be installed without any GitHub configuration. Only developers need this toolchain.

The Rust core is independent of Tauri and can be tested on Linux. Its `convert` example is a developer fixture helper; it is not required by the desktop app. The workspace-local `.local/` directory and all model files are ignored by Git.

See [conversion policy](docs/CONVERSION.md), [release setup](docs/RELEASING.md), and [validation record](docs/VALIDATION.md). Version 0.1.0 has been published; end-to-end automatic updating between installed versions has not yet been tested.
