# Conversion behavior and limits

The converter starts with the resolved **installed** Kobra X machine and native process settings, then applies a reviewed process allowlist. It does not copy a source project wholesale and rename its printer. Printer commands come from the installed target preset. Installed profiles are trusted local configuration; imported project archives are untrusted.

## Settings

`policy.rs` explicitly lists compatible quality/structure and motion keys. Supported values are copied even when they equal the source printer's factory defaults: a designer's one wall and 5% infill must survive. The source `different_settings_to_system` list is not a reliable complete description of design intent. The resulting project records transferred process differences.

Walls, layers, infill, support choices/distances, brim, seam position, and XY compensation are retained where supported. No mesh repair, automatic scaling, support addition, or wall-count clamping occurs. Invalid counts or incompatible layer/line widths are rejected visibly. Unsupported quality keys are reported. Object, modifier, plate, and layer-range overrides pass through the same policy, rather than bypassing it.

Absolute speeds, accelerations, and jerk are capped by the native process and applicable machine limits. Percentage motion settings fall back to native defaults with a report entry. Recent Bambu exports can contain Standard/High Flow alternatives for a single physical nozzle; the slower positive motion alternative is used before capping. Conflicting quality alternatives are rejected. Zero automatic motion values retain native behavior.

Bambu automatic tree-support wall count `-1` becomes Orca's automatic `0`, with a report entry. Bambu automatic initial support/raft expansion `-1` has no direct Orca equivalent and uses the target default, flagged for review. Other automatic sentinel values are accepted only where reviewed. No source G-code, filament scripts, executable post-processing commands, or embedded printer/process/filament presets are transferred. Plain pause messages and color/tool-change events survive; custom/template/legacy per-layer instructions are dropped and reported.

Native target filament defaults are selected using each slot's declared material family. All source flow, temperature, cooling, pressure advance, retraction, and filament scripts are replaced. Color arrays and slot indices are retained. Unsupported or undeclared materials fail instead of assuming PLA. Native defaults are starting points, not a claim to identify the actual filament spool. A user must select their actual filament profiles after import.

## Geometry and plates

The mesh parts are retained byte for byte, including triangle painting attributes. Geometry validation streams XML, resolves nested component references across model parts, composes transforms, and computes bounds from transformed vertices. Assembly metadata, modifiers, object identity, and variable layer-height data are retained. Bounds checks include the actual orientation and scale, target height, and bed exclusions.

Plate assignments are read from `model_settings.config`; multiple instances of one object use their per-object instance indices. Orca's plate layout uses `ceil(sqrt(plate_count))` columns and a 1.2 bed-size stride. Objects migrate from the source grid to the target grid together. Within each plate, XY recentering occurs only when its group bounds fail the target bed check. Relative placement and Z remain unchanged. Exclusion tests conservatively compare group bounding rectangles; a false rejection is possible for unusual layouts. This version supports rectangular beds and rectangular exclusions only. Arrange difficult projects in the source slicer first.

Source sliced bounding boxes and slice-info are removed as stale. Thumbnails remain source documentation and are not regenerated. Prime-tower and brim/support extents require the sliced Preview check: mesh fit alone does not prove toolpath fit. Variable layer values outside the native nozzle range fail rather than being silently clamped. Painting metadata is retained; actual rendering and slicing compatibility must still be checked in the target slicer.

## Archive handling

Limits: 512 MiB input/expanded total, 128 MiB per entry, 4096 entries, 2000:1 compression ratio, 64 XML/component nesting levels, 4 million XML elements per part, 3 million vertices/triangles, 20 million expanded vertex evaluations, 20,000 objects, 10,000 build instances, and 100 plates. No archive member is extracted to an input-controlled filesystem path. Absolute/traversal/drive/duplicate paths, links, encryption, DTDs/entities, external package relationships, invalid numeric geometry, and component cycles are refused. Embedded content is never executed or rendered in the UI.

Both output files are prepared in the source directory. The 3MF is reopened and every entry compared before commit, and geometry counts are rechecked. Final publication uses no-clobber filesystem operations. If committing the project fails, its just-created report is removed. Originals and earlier conversions are never overwritten. Crash recovery is limited to avoiding damage to originals; a force-terminated process can leave a temporary file or orphan report.

## Reference review

[ferdytao's scripts](https://github.com/ferdytao/makerworld-to-anycubic-presets) informed the allowlist and native-profile transfer approach. Their MIT notice is retained in `THIRD_PARTY_NOTICES.md` and bundled with the app. The scripts are not executed. Important changes include complete 3MF output, inheritance-cycle/missing-parent errors, aggregate archive limits, transform-aware bounds, filtering nested overrides, complete native filament replacement, preservation of source quality defaults, and atomic no-overwrite writes. The reference's mesh extents ignore rotation and scale, and its output is a separate process preset, so those parts were not suitable as this app's implementation.
