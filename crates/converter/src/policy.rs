use crate::profiles::{num, number, scalar, string, Target};
use anyhow::{ensure, Context, Result};
use serde_json::Value;

// Reviewed process-only choices. Never use wildcard speed/filament/script copying.
pub const QUALITY: &[&str] = &[
    "wall_loops",
    "top_shell_layers",
    "bottom_shell_layers",
    "top_shell_thickness",
    "bottom_shell_thickness",
    "sparse_infill_density",
    "sparse_infill_pattern",
    "internal_solid_infill_pattern",
    "top_surface_pattern",
    "bottom_surface_pattern",
    "infill_direction",
    "infill_wall_overlap",
    "wall_generator",
    "wall_sequence",
    "seam_position",
    "detect_thin_wall",
    "detect_overhang_wall",
    "ensure_vertical_shell_thickness",
    "enable_support",
    "support_type",
    "support_style",
    "support_threshold_angle",
    "support_on_build_plate_only",
    "support_base_pattern",
    "support_base_pattern_spacing",
    "support_interface_top_layers",
    "support_interface_bottom_layers",
    "support_interface_spacing",
    "support_interface_pattern",
    "support_top_z_distance",
    "support_bottom_z_distance",
    "support_object_xy_distance",
    "support_angle",
    "tree_support_branch_angle",
    "tree_support_branch_diameter",
    "tree_support_branch_distance",
    "tree_support_wall_count",
    "tree_support_tip_diameter",
    "raft_layers",
    "raft_first_layer_expansion",
    "raft_contact_distance",
    "brim_type",
    "brim_width",
    "brim_object_gap",
    "skirt_loops",
    "skirt_distance",
    "skirt_height",
    "ironing_type",
    "ironing_flow",
    "ironing_spacing",
    "ironing_pattern",
    "ironing_inset",
    "ironing_angle",
    "bridge_flow",
    "bridge_angle",
    "elefant_foot_compensation",
    "xy_hole_compensation",
    "xy_contour_compensation",
    "enable_overhang_speed",
    "only_one_wall_top",
    "only_one_wall_first_layer",
    "spiral_mode",
    "support_critical_regions_only",
    "support_filament",
    "support_interface_filament",
    "wall_filament",
    "sparse_infill_filament",
    "solid_infill_filament",
    "layer_height",
    "initial_layer_print_height",
    "line_width",
    "outer_wall_line_width",
    "inner_wall_line_width",
    "top_surface_line_width",
    "sparse_infill_line_width",
    "internal_solid_infill_line_width",
    "support_line_width",
    "initial_layer_line_width",
];
pub const MOTION: &[&str] = &[
    "outer_wall_speed",
    "inner_wall_speed",
    "sparse_infill_speed",
    "internal_solid_infill_speed",
    "top_surface_speed",
    "gap_infill_speed",
    "support_speed",
    "support_interface_speed",
    "initial_layer_speed",
    "initial_layer_infill_speed",
    "travel_speed",
    "travel_speed_z",
    "bridge_speed",
    "overhang_1_4_speed",
    "overhang_2_4_speed",
    "overhang_3_4_speed",
    "overhang_4_4_speed",
    "ironing_speed",
    "default_acceleration",
    "outer_wall_acceleration",
    "inner_wall_acceleration",
    "top_surface_acceleration",
    "sparse_infill_acceleration",
    "internal_solid_infill_acceleration",
    "initial_layer_acceleration",
    "travel_acceleration",
    "bridge_acceleration",
    "default_jerk",
    "outer_wall_jerk",
    "inner_wall_jerk",
    "infill_jerk",
    "top_surface_jerk",
    "initial_layer_jerk",
    "travel_jerk",
];
pub fn alias(k: &str) -> &str {
    if k == "ironing_direction" {
        "ironing_angle"
    } else {
        k
    }
}
pub fn supported(k: &str) -> bool {
    QUALITY.contains(&alias(k)) || MOTION.contains(&alias(k))
}
fn atom(v: &Value) -> Result<String> {
    let v = scalar(v);
    ensure!(
        !v.is_array() && !v.is_object() && !v.is_null(),
        "Unsupported multi-value process setting"
    );
    let s = string(v);
    ensure!(
        !s.is_empty()
            && s.len() <= 100
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || " .%()_-+/".contains(c)),
        "Invalid process value"
    );
    Ok(s)
}
pub fn transfer(k: &str, v: &Value, t: &Target, notes: &mut Vec<String>) -> Result<Option<Value>> {
    let key = alias(k);
    if !supported(key) {
        return Ok(None);
    }
    if !t.process.contains_key(key) {
        notes.push(format!(
            "Review: {k} is not exposed by the installed target process and was dropped"
        ));
        return Ok(None);
    }
    // Recent Bambu exports store Standard/High Flow alternatives even for one
    // physical nozzle. For motion only, use the slowest declared alternative,
    // then cap against native limits. Never choose between conflicting quality values.
    let collapsed;
    let val = if let Some(a) = v.as_array() {
        ensure!(!a.is_empty(), "Empty process setting {k}");
        if a.iter().all(|x| x == &a[0]) {
            &a[0]
        } else if MOTION.contains(&key) {
            let percent = string(&a[0]).ends_with('%');
            ensure!(
                a.iter().all(|v| string(v).ends_with('%') == percent),
                "{k} mixes absolute and percentage motion values"
            );
            let values = a
                .iter()
                .map(|v| {
                    let s = string(v);
                    let n: f64 = s.trim_end_matches('%').parse()?;
                    ensure!(n.is_finite() && n >= 0.0, "Invalid motion alternative");
                    Ok(n)
                })
                .collect::<Result<Vec<f64>>>()?;
            let n = values
                .into_iter()
                .filter(|n| *n > 0.0)
                .fold(f64::INFINITY, f64::min);
            collapsed = Value::String(format!(
                "{}{}",
                if n.is_finite() { n } else { 0.0 },
                if percent { "%" } else { "" }
            ));
            notes.push(format!(
                "{k}: used the slowest source nozzle-variant motion value before target clamping"
            ));
            &collapsed
        } else {
            anyhow::bail!(
                "{k} has conflicting nozzle values; this project needs review in its source slicer"
            );
        }
    } else {
        v
    };
    let mut s = atom(val).with_context(|| format!("Invalid value for {k}"))?;
    if key == "ironing_pattern" && s == "zig-zag" {
        s = "rectilinear".into();
    }
    if key == "ensure_vertical_shell_thickness" && s == "enabled" {
        s = "ensure_all".into();
    }
    if key == "tree_support_wall_count" && s == "-1" {
        s = "0".into();
        notes.push(
            "Review: translated Bambu automatic support wall count (-1) to native automatic (0)"
                .into(),
        );
    }
    if key == "raft_first_layer_expansion" && s == "-1" {
        notes.push("Review: Bambu automatic initial support/raft expansion has no native equivalent; used the installed target default".into());
        return Ok(None);
    }
    if key == "layer_height" || key == "initial_layer_print_height" {
        let n = number(&Value::String(s.clone()))?;
        ensure!(n >= num(&t.machine,"min_layer_height")? - 1e-6 && n <= num(&t.machine,"max_layer_height")? + 1e-6,
            "{k} ({n} mm) is outside the installed Kobra X layer range. Change it in the source slicer.");
    }
    if MOTION.contains(&key) {
        let pct = s.ends_with('%');
        let n: f64 = s.trim_end_matches('%').parse()?;
        ensure!(n.is_finite() && n >= 0.0, "Invalid motion setting {k}");
        let reference = string(t.process.get(key).unwrap());
        if pct || reference.ends_with('%') {
            notes.push(format!(
                "{k}: used native target motion default instead of an ambiguous percentage"
            ));
            return Ok(None);
        }
        let r = number(t.process.get(key).unwrap())?;
        let cap = if key.ends_with("acceleration") {
            num(
                &t.machine,
                if key == "travel_acceleration" {
                    "machine_max_acceleration_travel"
                } else {
                    "machine_max_acceleration_extruding"
                },
            )?
            .min(num(&t.machine, "machine_max_acceleration_x")?)
            .min(num(&t.machine, "machine_max_acceleration_y")?)
        } else if key.ends_with("jerk") {
            num(&t.machine, "machine_max_jerk_x")?.min(num(&t.machine, "machine_max_jerk_y")?)
        } else if key == "travel_speed_z" {
            num(&t.machine, "machine_max_speed_z")?
        } else {
            num(&t.machine, "machine_max_speed_x")?.min(num(&t.machine, "machine_max_speed_y")?)
        };
        // A zero is automatic/unlimited. Keep the native default instead of converting
        // the target's carefully tuned automatic rule into a source rule.
        if n == 0.0 {
            return Ok(None);
        }
        let limited = n.min(if r > 0.0 { r.min(cap) } else { cap });
        if limited != n {
            notes.push(format!(
                "{k}: limited {n} to {limited} using native target limits"
            ));
        }
        s = limited.to_string();
    } else if let Ok(n) = s.trim_end_matches('%').parse::<f64>() {
        ensure!(n.is_finite(), "Non-finite {k}");
        if key == "support_interface_bottom_layers" {
            ensure!(n >= -1.0, "Invalid automatic setting {k}");
        } else if !matches!(
            key,
            "xy_hole_compensation" | "xy_contour_compensation" | "ironing_angle"
        ) {
            ensure!(n >= 0.0, "Negative {k}");
        }
        if matches!(
            key,
            "wall_loops" | "top_shell_layers" | "bottom_shell_layers" | "raft_layers"
        ) {
            ensure!(
                n.fract() == 0.0 && n <= 1000.0,
                "Invalid layer or wall count {k}"
            );
        }
        if key == "sparse_infill_density" {
            ensure!(n <= 100.0, "Invalid infill percentage");
        }
        if key.ends_with("line_width") {
            let width = if s.ends_with('%') { n / 100.0 * 0.4 } else { n };
            ensure!(
                width == 0.0 || (0.24..=0.8).contains(&width),
                "{k} is not supported for a 0.4 mm nozzle; change it in the source slicer"
            );
        }
    }
    Ok(Some(Value::String(s)))
}

pub fn is_metadata(k: &str) -> bool {
    matches!(
        k,
        "name"
            | "extruder"
            | "matrix"
            | "volume_type"
            | "mesh_stat"
            | "face_count"
            | "object_id"
            | "instance_id"
            | "identify_id"
            | "locked"
            | "source_file"
            | "source_object_id"
            | "source_volume_id"
            | "source_offset_x"
            | "source_offset_y"
            | "source_offset_z"
            | "source_in_inches"
            | "source_in_meters"
            | "text"
            | "volume_id"
            | "subtype"
            | "plater_id"
            | "plater_name"
            | "filament_map_mode"
            | "filament_maps"
            | "filament_volume_maps"
            | "thumbnail_file"
            | "thumbnail_no_light_file"
            | "top_file"
            | "pick_file"
            | "plate_name"
    )
}

pub fn sanitize_overrides(
    text: &str,
    target: &Target,
    slots: usize,
    notes: &mut Vec<String>,
) -> Result<String> {
    let doc = roxmltree::Document::parse(text)?;
    let mut edits = Vec::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("metadata")) {
        let Some(k) = node.attribute("key") else {
            continue;
        };
        let v = node.attribute("value").unwrap_or("");
        if is_metadata(k) {
            if k == "extruder" {
                let n: usize = v.parse()?;
                ensure!(n <= slots, "Object uses missing filament slot {n}");
            }
            continue;
        }
        match transfer(k, &Value::String(v.into()), target, notes)? {
            Some(value) => edits.push((
                node.range(),
                format!(
                    "<metadata key=\"{}\" value=\"{}\"/>",
                    alias(k),
                    string(&value)
                ),
            )),
            None => {
                notes.push(format!("Dropped object/part/plate setting {k}: source machine, filament, or unsupported setting"));
                edits.push((node.range(), String::new()));
            }
        }
    }
    let mut out = text.to_string();
    for (range, replacement) in edits.into_iter().rev() {
        out.replace_range(range, &replacement);
    }
    Ok(out)
}
