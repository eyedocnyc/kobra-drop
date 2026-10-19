pub mod archive;
pub mod geometry;
pub mod policy;
pub mod profiles;

use anyhow::{ensure, Context, Result};
use profiles::{merge_native, scalar, string, Catalog, Settings, Target};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Conversion {
    pub output: String,
    pub report: String,
    pub summary: String,
    pub warnings: Vec<String>,
    pub details: String,
}
#[derive(Serialize, Debug)]
pub struct Audit {
    pub target: String,
    pub vertices: usize,
    pub triangles: usize,
    pub filament_slots: usize,
    pub plates: Vec<geometry::PlateResult>,
    pub notes: Vec<String>,
    pub preserved_parts: Vec<String>,
}

fn filament_config(
    source: &Settings,
    target: &Target,
    out: &mut Settings,
    notes: &mut Vec<String>,
) -> Result<usize> {
    let types = source
        .get("filament_type")
        .and_then(Value::as_array)
        .context(
        "The project must declare the material type of each filament slot; PLA is never assumed",
    )?;
    ensure!(
        !types.is_empty() && types.len() <= 64,
        "Unsupported filament slot count (1–64 required)"
    );
    let slots = types.len();
    if let Some(ids) = source.get("filament_settings_id").and_then(Value::as_array) {
        ensure!(
            ids.len() == slots,
            "Filament slot metadata disagrees about slot count"
        );
    }
    let native = types
        .iter()
        .map(|t| target.material(t.as_str().context("Invalid filament type")?))
        .collect::<Result<Vec<_>>>()?;
    let keys = native
        .iter()
        .flat_map(|(_, f)| f.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        if matches!(
            key.as_str(),
            "type"
                | "name"
                | "from"
                | "inherits"
                | "instantiation"
                | "version"
                | "setting_id"
                | "filament_id"
                | "is_custom_defined"
                | "compatible_printers"
                | "compatible_prints"
                | "compatible_printers_condition"
                | "compatible_prints_condition"
        ) {
            continue;
        }
        // All filament values come from native defaults, including flow, temperature,
        // cooling, pressure advance and native scripts. No source filament overrides.
        let values = native
            .iter()
            .map(|(_, f)| {
                let v = f
                    .get(&key)
                    .with_context(|| format!("Native material profiles disagree: missing {key}"))?;
                Ok(scalar(v).clone())
            })
            .collect::<Result<Vec<Value>>>()?;
        out.insert(key, Value::Array(values));
    }
    out.insert(
        "filament_settings_id".into(),
        json!(native.iter().map(|(n, _)| n).collect::<Vec<_>>()),
    );
    // filament_id is scalar preset metadata, not a filament option. Serializing
    // it as an array makes Anycubic abort its entire JSON import at that key.
    // A merged project carries these identifiers in filament_ids instead.
    out.insert(
        "filament_ids".into(),
        json!(native
            .iter()
            .map(|(_, f)| f.get("filament_id").and_then(Value::as_str).unwrap_or(""))
            .collect::<Vec<_>>()),
    );
    for k in [
        "filament_colour",
        "filament_multi_colour",
        "filament_colour_type",
    ] {
        if let Some(v) = source.get(k) {
            let a = v.as_array().context("Invalid filament color list")?;
            ensure!(a.len() == slots, "Color list does not match filament slots");
            ensure!(
                a.iter().all(|v| v
                    .as_str()
                    .map(|s| s.len() <= 256
                        && s.chars()
                            .all(|c| c.is_ascii_hexdigit() || "#;, ".contains(c)))
                    .unwrap_or(false)),
                "Invalid filament colors"
            );
            out.insert(k.into(), v.clone());
        }
    }
    out.insert(
        "filament_map".into(),
        json!((1..=slots).map(|n| n.to_string()).collect::<Vec<_>>()),
    );
    notes.push("Choose the actual filament profile for EVERY color slot after import. Native defaults match the declared material family only; they do not identify the loaded spool.".into());
    Ok(slots)
}

fn layer_profile(bytes: &[u8], target: &Target) -> Result<()> {
    let text = std::str::from_utf8(bytes)?;
    let min = profiles::num(&target.machine, "min_layer_height")?;
    let max = profiles::num(&target.machine, "max_layer_height")?;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let (id, values) = line
            .split_once('|')
            .context("Unsupported variable layer profile format")?;
        ensure!(
            id.starts_with("object_id="),
            "Unsupported variable layer object mapping"
        );
        let numbers = values
            .split(';')
            .filter(|s| !s.is_empty())
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(numbers.len() % 2 == 0, "Malformed variable layer profile");
        let mut last = -1.;
        for pair in numbers.chunks(2) {
            ensure!(pair[0].is_finite() && pair[0]>=last && pair[1].is_finite() && pair[1]>=min-1e-6&&pair[1]<=max+1e-6,"Variable layer heights are outside the target range or malformed; change them in the source slicer");
            last = pair[0];
        }
    }
    Ok(())
}

fn filter_layer_events(text: &str, slots: usize, notes: &mut Vec<String>) -> Result<String> {
    let doc = roxmltree::Document::parse(text)?;
    let mut edits = Vec::new();
    ensure!(
        doc.root_element().has_tag_name("custom_gcodes_per_layer"),
        "Unsupported custom layer instructions format"
    );
    ensure!(
        doc.descendants()
            .filter(|n| n.is_element())
            .all(|n| matches!(
                n.tag_name().name(),
                "custom_gcodes_per_layer" | "plate" | "plate_info" | "mode" | "layer"
            )),
        "Unknown custom layer instruction structure; save a supported project first"
    );
    for node in doc.descendants().filter(|n| n.has_tag_name("layer")) {
        let kind = node.attribute("type").unwrap_or("");
        // Official CustomGCode enum: 0=color change, 1=pause, 2=tool change;
        // template and custom types carry arbitrary commands and must be removed.
        if matches!(kind, "0" | "1" | "2") {
            let z: f64 = node
                .attribute("top_z")
                .context("Layer event missing height")?
                .parse()?;
            let extruder: usize = node.attribute("extruder").unwrap_or("0").parse()?;
            ensure!(
                z.is_finite() && z >= 0. && extruder <= slots,
                "Invalid layer event"
            );
            let color = node.attribute("color").unwrap_or("");
            ensure!(
                color.len() <= 9 && color.chars().all(|c| c.is_ascii_hexdigit() || c == '#'),
                "Invalid layer color"
            );
            // Retain plain pause messages, while excluding command-like content
            // and all source executable attributes.
            let message = if kind == "1" {
                node.attribute("extra").unwrap_or("")
            } else {
                ""
            };
            let command = message.trim_start().as_bytes();
            let command_like = command.len() > 1
                && matches!(command[0], b'G' | b'M' | b'T')
                && command[1].is_ascii_digit();
            let message = if message.len() <= 256
                && !message.chars().any(char::is_control)
                && !command_like
            {
                message
            } else {
                notes.push("Dropped unsafe pause message; pause retained for review".into());
                ""
            };
            let escaped = message
                .replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            edits.push((node.range(),format!("<layer top_z=\"{z}\" type=\"{kind}\" extruder=\"{extruder}\" color=\"{color}\" extra=\"{escaped}\"/>")));
            notes.push(format!(
                "Preserved non-G-code layer event type {kind} at {z} mm; review in Preview"
            ));
        } else {
            edits.push((node.range(), String::new()));
            notes.push("Dropped custom or legacy per-layer G-code; review the designer's instructions before slicing".into());
        }
    }
    let mut out = text.to_string();
    for (r, s) in edits.into_iter().rev() {
        out.replace_range(r, &s);
    }
    Ok(out)
}

pub fn transform(entries: &mut archive::Entries, target: &Target) -> Result<Audit> {
    let source: Settings=serde_json::from_slice(entries.get("Metadata/project_settings.config").context("Missing editable project settings; G-code packages and geometry-only archives cannot be converted")?)?;
    let mut notes = Vec::new();
    let mut out = Settings::new();
    merge_native(&mut out, &target.process);
    merge_native(&mut out, &target.machine);
    let slots = filament_config(&source, target, &mut out, &mut notes)?;
    let mut applied = Vec::new();
    for (key, value) in &source {
        if let Some(value) = policy::transfer(key, value, target, &mut notes)? {
            if key.ends_with("_filament") {
                let n: usize = string(&value).parse()?;
                ensure!(
                    n <= slots,
                    "Setting {key} references a missing filament slot"
                );
            }
            let k = policy::alias(key).to_string();
            applied.push(k.clone());
            out.insert(k, value);
        } else if !target.machine.contains_key(key)
            && !key.starts_with("filament_")
            && !out.contains_key(key)
        {
            notes.push(format!(
                "Dropped project setting {key}: not in the reviewed compatible process allowlist"
            ));
        }
    }
    // Never permit embedded scripts or executable post-process paths from the input.
    out.insert("post_process".into(), json!([]));
    out.insert("name".into(), json!("project_settings"));
    if let Some(version) = target.machine.get("version").and_then(Value::as_str) {
        out.insert("version".into(), json!(version));
    }
    out.insert(
        "print_compatible_printers".into(),
        json!([profiles::MACHINE]),
    );
    out.insert("printer_settings_id".into(), json!(profiles::MACHINE));
    out.insert("print_settings_id".into(), json!(target.process_name));
    let mut diff = vec![applied.join(";")];
    diff.resize(slots + 2, String::new());
    out.insert("different_settings_to_system".into(), json!(diff));
    out.insert("inherits_group".into(), json!(vec![""; slots + 2]));
    let mut changed = Vec::new();
    let mut remove = Vec::new();
    for (name, bytes) in entries.iter() {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".gcode")
            || lower.ends_with(".bgcode")
            || lower.contains("slice_info")
            || lower.starts_with("metadata/filament_settings_")
            || lower.starts_with("metadata/machine_settings_")
            || lower.starts_with("metadata/process_settings_")
            || lower.starts_with("metadata/print_setting_")
        {
            remove.push(name.clone());
            notes.push(format!("Removed stale printer data: {name}"));
            continue;
        }
        if name == "Metadata/model_settings.config" || name == "Metadata/layer_config_ranges.xml" {
            let text = std::str::from_utf8(bytes)?;
            let filtered = policy::sanitize_overrides(text, target, slots, &mut notes)?;
            // Layer-range configs also use <option opt_key="..."> syntax.
            let filtered = sanitize_options(&filtered, target, &mut notes)?;
            changed.push((name.clone(), filtered.into_bytes()));
        } else if name == "Metadata/layer_heights_profile.txt" {
            layer_profile(bytes, target)?;
        } else if name == "Metadata/custom_gcode_per_layer.xml" {
            changed.push((
                name.clone(),
                filter_layer_events(std::str::from_utf8(bytes)?, slots, &mut notes)?.into_bytes(),
            ));
        } else if lower.ends_with(".rels") {
            let doc = roxmltree::Document::parse(std::str::from_utf8(bytes)?)?;
            ensure!(!doc.descendants().any(|n|n.attribute("TargetMode")==Some("External")),"External package relationships are unsupported; save a self-contained project first");
            for relation in doc.descendants().filter(|n| n.has_tag_name("Relationship")) {
                archive::relationship_target(
                    name,
                    relation
                        .attribute("Target")
                        .context("Package relationship has no target")?,
                )?;
            }
        } else if lower.ends_with(".config") && name != "Metadata/project_settings.config" {
            remove.push(name.clone());
            notes.push(format!("Dropped unsupported configuration part {name}"));
        } else if !(name.ends_with(".model")
            || name.ends_with(".rels")
            || name == "[Content_Types].xml"
            || name == "Metadata/cut_information.xml"
            || name == "Metadata/brim_ear_points.txt"
            || name == "Metadata/filament_sequence.json"
            || lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".webp")
            || lower.starts_with("metadata/plate_") && lower.ends_with(".json"))
        {
            remove.push(name.clone());
            notes.push(format!(
                "Dropped unsupported archive part {name}; review required"
            ));
        }
    }
    for name in remove {
        entries.remove(&name);
    }
    for (name, bytes) in changed {
        entries.insert(name, bytes);
    }
    let geometry = geometry::inspect(entries)?;
    let plates = geometry::place(entries, &geometry, &source, &target.machine)?;
    for plate in &plates {
        if plate.translation.iter().any(|n| n.abs() > 1e-6) {
            notes.push(format!("Moved plate {} objects together by {:.3}, {:.3} mm; dimensions and relative placement retained",plate.plate,plate.translation[0],plate.translation[1]));
        }
    }
    // Slice bounding boxes/thumbnails refer to the source machine. Retain images as
    // documentation but discard stale sliced plate JSON, not plate assignments.
    entries.retain(|name, _| !(name.starts_with("Metadata/plate_") && name.ends_with(".json")));
    entries.insert("Metadata/project_settings.config".into(), {
        validate_project_metadata(&out, slots)?;
        serde_json::to_vec_pretty(&out)?
    });
    notes.push("Choose your filament, slice, and check Preview. Conversion does not certify printability; review supports, colors, clearances, and purge-tower placement.".into());
    let preserved_parts = entries
        .iter()
        .filter(|(n, _)| n.ends_with(".model") || n.ends_with("layer_heights_profile.txt"))
        .map(|(n, b)| {
            format!(
                "{n}: SHA-256 {}",
                Sha256::digest(b)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            )
        })
        .collect();
    Ok(Audit {
        target: profiles::MACHINE.into(),
        vertices: geometry.vertices,
        triangles: geometry.triangles,
        filament_slots: slots,
        plates,
        notes,
        preserved_parts,
    })
}

/// These names are handled as scalar metadata before the slicer reaches its
/// regular option parser. A wrong JSON type aborts import, leaving later keys
/// (including nozzle temperatures and wall counts) at slicer defaults.
pub fn validate_project_metadata(settings: &Settings, slots: usize) -> Result<()> {
    for key in [
        "version",
        "name",
        "type",
        "setting_id",
        "filament_id",
        "from",
        "description",
        "instantiation",
        "is_custom_defined",
        "url",
        "inherits",
    ] {
        if let Some(value) = settings.get(key) {
            ensure!(
                value.is_string(),
                "Project metadata {key} must be a string; the target slicer would abort import"
            );
        }
    }
    for key in ["filament_settings_id", "filament_ids"] {
        let values = settings
            .get(key)
            .and_then(Value::as_array)
            .with_context(|| format!("Missing project {key}"))?;
        ensure!(
            values.len() == slots && values.iter().all(Value::is_string),
            "Invalid project color-slot metadata {key}"
        );
    }
    Ok(())
}

fn sanitize_options(text: &str, target: &Target, notes: &mut Vec<String>) -> Result<String> {
    let doc = roxmltree::Document::parse(text)?;
    let mut edits = Vec::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("option")) {
        let key = node
            .attribute("opt_key")
            .context("Unknown layer range option syntax")?;
        let val = node.text().unwrap_or("");
        let replacement = match policy::transfer(key, &Value::String(val.into()), target, notes)? {
            Some(v) => format!(
                "<option opt_key=\"{}\">{}</option>",
                policy::alias(key),
                string(&v)
            ),
            None => {
                notes.push(format!("Dropped layer-range override {key}"));
                String::new()
            }
        };
        edits.push((node.range(), replacement));
    }
    let mut out = text.to_string();
    for (r, s) in edits.into_iter().rev() {
        out.replace_range(r, &s);
    }
    Ok(out)
}

pub fn convert(
    input: &Path,
    profile_root: Option<&Path>,
    progress: impl Fn(&str),
) -> Result<Conversion> {
    ensure!(
        input
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("3mf"))
            .unwrap_or(false),
        "Choose an editable .3mf project"
    );
    progress("Reading installed Kobra X profiles…");
    let target = Catalog::load(&profiles::discover(profile_root))?.target()?;
    progress("Checking the project archive…");
    let mut entries = archive::read(input)?;
    progress("Converting settings and checking transformed geometry…");
    let audit = transform(&mut entries, &target)?;
    let details = serde_json::to_string_pretty(&audit)?;
    let summary = format!(
        "{} plate(s) · {} color slot(s) · {} triangles. Geometry and painting retained.",
        audit.plates.len(),
        audit.filament_slots,
        audit.triangles
    );
    let reviews = audit
        .notes
        .iter()
        .filter(|n| n.starts_with("Dropped") || n.starts_with("Review:"))
        .count();
    let mut warnings=vec!["Choose the actual filament profile for every color slot. Slice and check Preview before printing.".into()];
    if reviews > 0 {
        warnings.push(format!("{reviews} unsupported settings or instructions were dropped. Read Conversion details before slicing."));
    }
    let report_text=format!("Kobra Drop conversion report\n\n{summary}\n\nChoose your filament, slice, and check Preview.\n\nNative material defaults replace ALL source filament overrides. They are placeholders for your actual spool profiles.\n\n{details}\n");
    let parent = input.parent().context("The file has no parent folder")?;
    let stem = input.file_stem().unwrap().to_string_lossy();
    progress("Writing and verifying the converted project…");
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .context("Cannot write next to the source file. Move it to a writable folder.")?;
    archive::write(temp.as_file_mut(), &entries)?;
    let verified = archive::read(temp.path())?;
    ensure!(verified == entries, "Output archive verification failed");
    let after = geometry::inspect(&verified)?;
    ensure!(
        after.vertices == audit.vertices && after.triangles == audit.triangles,
        "Output geometry verification failed"
    );
    let mut report_temp = tempfile::NamedTempFile::new_in(parent)?;
    report_temp.write_all(report_text.as_bytes())?;
    report_temp.as_file_mut().sync_all()?;
    for index in 0..10000 {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!("_{index}")
        };
        let output = parent.join(format!("{stem}_KobraX{suffix}.3mf"));
        let report = parent.join(format!("{stem}_KobraX{suffix}.report.txt"));
        if output.exists() || report.exists() {
            continue;
        }
        // persist_noclobber uses a no-overwrite filesystem operation, including
        // against a file created after the above existence check.
        match report_temp.persist_noclobber(&report) {
            Ok(_) => {}
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                report_temp = e.file;
                continue;
            }
            Err(e) => return Err(e.error.into()),
        }
        match temp.persist_noclobber(&output) {
            Ok(_) => {
                return Ok(Conversion {
                    output: output.to_string_lossy().into(),
                    report: report.to_string_lossy().into(),
                    summary,
                    warnings,
                    details,
                })
            }
            Err(e) => {
                let _ = std::fs::remove_file(&report);
                return Err(e.error)
                    .context("Could not commit the converted project; original is unchanged");
            }
        }
    }
    anyhow::bail!("Too many existing conversions next to this file")
}

pub fn convert_with_catalog(input: &Path, output_dir: &Path, catalog: &Catalog) -> Result<PathBuf> {
    // Developer-only fixture helper: never used by the desktop UI.
    let mut entries = archive::read(input)?;
    transform(&mut entries, &catalog.target()?)?;
    let out = output_dir.join("fixture_KobraX.3mf");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)?;
    archive::write(&mut file, &entries)?;
    Ok(out)
}
