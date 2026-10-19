# Platform support

Windows x64 is the current supported implementation. The converter core is portable Rust and is already exercised on Linux, but that does not establish macOS desktop or slicer compatibility.

## macOS assessment

GitHub Actions can build Apple Silicon and Intel Tauri packages on macOS runners. Adding a runner alone is insufficient for this app because the current integration deliberately targets Windows:

- Discover the macOS Anycubic Slicer Next application bundle and installed/cached profiles instead of Windows Program Files and AppData directories.
- Launch the native `.app` and update the Locate slicer dialog, which currently selects an `.exe`.
- Handle Finder file-open events; Windows command-line arguments do not cover macOS application-open behavior.
- Add macOS icons, app/DMG packaging, architecture targets, and signed updater manifest entries. The current release manifest contains only Windows x64.
- Configure Apple Developer ID signing and notarization for distribution without manual Gatekeeper exceptions. Tauri updater signatures remain a separate requirement.
- Test actual Mac import, silk-profile selection, slicing, installation, relaunch, and a signed update on supported Mac hardware. Build success alone is insufficient evidence.

Recommended rollout: finish the Windows installation/update release checks, then introduce macOS as a beta after the platform integration and actual Mac validation. There is no fundamental conversion-engine obstacle to supporting both platforms, and Mac releases need not delay independently validated Windows releases.

Official references checked for this assessment:

- [Tauri GitHub Actions builds for Apple Silicon and Intel](https://v2.tauri.app/distribute/pipelines/github/)
- [Anycubic's Mac x86 and ARM slicer downloads](https://store.anycubic.com/pages/firmware-software)
- [macOS application bundles](https://v2.tauri.app/distribute/macos-application-bundle/)
- [Apple signing, notarization, and ad-hoc distribution limitations](https://v2.tauri.app/distribute/sign/macos/)

macOS support has not been implemented or advertised as tested in this version.
