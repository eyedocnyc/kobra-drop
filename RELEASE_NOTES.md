# Kobra Drop 0.1.0

Initial Windows x64 release candidate for Anycubic Slicer Next and the Anycubic Kobra X with a 0.4 mm nozzle.

- Drop or choose an editable MakerWorld/Bambu 3MF and convert it entirely locally.
- Preserve mesh geometry, component transforms, plate assignments, painting, supported object/modifier settings, and compatible variable layer heights.
- Use installed native Kobra X machine and material profiles. Preserve compatible design settings and cap transferred motion values.
- Save a separate verified project and readable report beside the original.
- Open the result in Anycubic Slicer Next. Choose the actual filament profile, slice, and check Preview.
- Include a current-user Windows installer and prepared GitHub signed-update automation.

The corrected dragon-egg fixture passed user-reported silk PLA selection and slicing, as well as an automated import/export check with the installed slicer. The filament metadata import defect found during manual testing is fixed in this candidate.

Local packages are unsigned and have automatic-update endpoints unconfigured. Public signed updates require the destination repository and production signing configuration. Installer/update testing remains a release gate. No release has been published.

macOS is a proposed follow-up platform and is not supported by this package. See [platform assessment](docs/PLATFORMS.md).
