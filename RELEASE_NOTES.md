# Kobra Drop 0.1.1

Windows x64 maintenance release for Anycubic Slicer Next and Kobra X 0.4 mm.

- Report designer settings replaced with differing native defaults.
- Preserve valid layer-range filament/color assignments.
- Exclude identified modifier, negative, and support-helper volumes from printable bounds while validating their geometry.
- Keep conversion results visible until opened or replaced when an update is ready.
- Allow 30 minutes for update downloads in this version and later.
- Narrow signing-secret exposure and pin GitHub Actions dependencies.

Version 0.1.0 retains its two-minute update download timeout. If automatic updating fails on a slower connection, download and run this installer manually. Preferences and converted files are retained.

Automated conversion, updater, release-configuration, and frontend checks passed. End-to-end installed updating and nested helper-volume layouts have not been verified. The installer uses Tauri updater signatures; Windows Authenticode signing is not configured.

Choose your actual filament, slice, and check Preview after importing a converted project.
