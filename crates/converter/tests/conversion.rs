use kobra_converter::{
    archive::{self, Entries},
    geometry,
    profiles::{Catalog, Settings, MACHINE},
    transform,
};
use serde_json::{json, Value};

fn settings(v: Value) -> Settings {
    v.as_object().unwrap().clone()
}
fn catalog() -> Catalog {
    let mut c = Catalog::default();
    c.items.insert(("machine".into(),"base".into()),settings(json!({"printable_area":["0x0","260x0","260x260","0x260"],"printable_height":"260","bed_exclude_area":[],"nozzle_diameter":["0.4"],"min_layer_height":["0.08"],"max_layer_height":["0.28"],"machine_max_speed_x":["450"],"machine_max_speed_y":["450"],"machine_max_speed_z":["12"],"machine_max_acceleration_extruding":["6500"],"machine_max_acceleration_travel":["10000"],"machine_max_acceleration_x":["10000"],"machine_max_acceleration_y":["10000"],"machine_max_jerk_x":["20"],"machine_max_jerk_y":["20"],"machine_start_gcode":"NATIVE_START","machine_end_gcode":"NATIVE_END"})));
    c.items.insert(("machine".into(),MACHINE.into()),settings(json!({"inherits":"base","printer_model":"Anycubic Kobra X","default_print_profile":"standard"})));
    c.items.insert(("process".into(),"standard".into()),settings(json!({"layer_height":"0.2","wall_loops":"2","sparse_infill_density":"15%","enable_support":"0","support_type":"normal(auto)","outer_wall_speed":"200","xy_hole_compensation":"0","support_top_z_distance":"0.2","ironing_angle":"-1","print_settings_id":"standard"})));
    for material in ["PLA", "PETG"] {
        c.items.insert(("filament".into(),format!("Native {material} @Anycubic Kobra X 0.4 nozzle")),settings(json!({"filament_type":[material],"filament_flow_ratio":["0.96"],"nozzle_temperature":["215"],"pressure_advance":["0.03"],"fan_max_speed":["80"],"filament_start_gcode":["NATIVE_FILAMENT"]})));
        c.items
            .get_mut(&(
                "filament".into(),
                format!("Native {material} @Anycubic Kobra X 0.4 nozzle"),
            ))
            .unwrap()
            .insert("filament_id".into(), json!(format!("NATIVE_{material}")));
    }
    c
}
fn fixture() -> Entries {
    let mut e = Entries::new();
    e.insert("[Content_Types].xml".into(), b"<Types/>".to_vec());
    e.insert("3D/3dmodel.model".into(),br#"<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" unit="millimeter" requiredextensions="p"><resources><object id="2"><components><component objectid="1" p:path="/3D/Objects/mesh.model" transform="0 1 0 -1 0 0 0 0 2 10 0 0"/></components></object></resources><build><item objectid="2" transform="1 0 0 0 1 0 0 0 1 100 100 0"/></build></model>"#.to_vec());
    e.insert("3D/Objects/mesh.model".into(),br#"<model unit="millimeter"><resources><object id="1"><mesh><vertices><vertex x="0" y="0" z="0"/><vertex x="10" y="0" z="0"/><vertex x="0" y="20" z="5"/></vertices><triangles><triangle v1="0" v2="1" v3="2" paint_color="0C" paint_supports="8" seam="1"/></triangles></mesh></object></resources></model>"#.to_vec());
    e.insert("Metadata/project_settings.config".into(),serde_json::to_vec(&json!({"printable_area":["0x0","256x0","256x256","0x256"],"filament_type":["PLA","PETG"],"filament_settings_id":["Bambu PLA","Bambu PETG"],"filament_colour":["#ff0000","#00ff00"],"filament_flow_ratio":["1.9","1.8"],"nozzle_temperature":["299","299"],"filament_start_gcode":["BAD","BAD"],"machine_start_gcode":"BAD_START","change_filament_gcode":"BAD_TOOLCHANGE","post_process":["evil.exe"],"wall_loops":"1","layer_height":"0.2","sparse_infill_density":"5%","enable_support":"0","outer_wall_speed":"900","xy_hole_compensation":"0.17"})).unwrap());
    e.insert("Metadata/model_settings.config".into(),br#"<config><object id="2"><metadata key="name" value="Test"/><metadata key="extruder" value="2"/><metadata key="wall_loops" value="1"/><metadata key="machine_start_gcode" value="EVIL"/><part id="1" subtype="modifier"><metadata key="support_top_z_distance" value="0.15"/><metadata key="outer_wall_speed" value="999"/><metadata key="nozzle_temperature" value="299"/></part></object><plate><metadata key="plater_id" value="1"/><model_instance><metadata key="object_id" value="2"/><metadata key="instance_id" value="0"/></model_instance></plate><assemble><assemble_item object_id="2" instance_id="0" transform="1 0 0 0 1 0 0 0 1 20 20 0"/></assemble></config>"#.to_vec());
    e.insert(
        "Metadata/layer_heights_profile.txt".into(),
        b"object_id=1|0;0.2;1;0.12;2;0.24\n".to_vec(),
    );
    e
}
#[test]
fn complete_project_preserves_components_painting_modifiers_layers_and_native_filaments() {
    let mut e = fixture();
    let original = e.clone();
    let t = catalog().target().unwrap();
    let before = geometry::inspect(&e).unwrap();
    assert_eq!(before.instances[0].bounds.min, [90., 100., 0.]);
    assert_eq!(before.instances[0].bounds.max, [110., 110., 10.]);
    let audit = transform(&mut e, &t).unwrap();
    assert_eq!(audit.filament_slots, 2);
    assert_eq!(
        e["3D/Objects/mesh.model"],
        original["3D/Objects/mesh.model"]
    );
    assert_eq!(e["3D/3dmodel.model"], original["3D/3dmodel.model"]);
    assert_eq!(
        e["Metadata/layer_heights_profile.txt"],
        original["Metadata/layer_heights_profile.txt"]
    );
    let p: Value = serde_json::from_slice(&e["Metadata/project_settings.config"]).unwrap();
    assert_eq!(p["machine_start_gcode"], "NATIVE_START");
    assert_eq!(p["wall_loops"], "1");
    assert_eq!(p["sparse_infill_density"], "5%");
    assert_eq!(p["enable_support"], "0");
    assert_eq!(p["outer_wall_speed"], "200");
    assert_eq!(p["xy_hole_compensation"], "0.17");
    assert_eq!(p["filament_flow_ratio"], json!(["0.96", "0.96"]));
    assert_eq!(p["nozzle_temperature"], json!(["215", "215"]));
    assert_eq!(p["filament_colour"], json!(["#ff0000", "#00ff00"]));
    assert_eq!(p["post_process"], json!([]));
    assert!(p.get("change_filament_gcode").is_none());
    let cfg = String::from_utf8(e["Metadata/model_settings.config"].clone()).unwrap();
    assert!(!cfg.contains("EVIL") && !cfg.contains("299"));
    assert!(
        cfg.contains("subtype=\"modifier\"")
            && cfg.contains("value=\"0.15\"")
            && cfg.contains("value=\"200\"")
    );
}
#[test]
fn multiple_plates_translate_together_and_keep_assignments() {
    let mut e = fixture();
    let model=String::from_utf8(e["3D/3dmodel.model"].clone()).unwrap().replace("</build>","<item objectid=\"2\" transform=\"1 0 0 0 1 0 0 0 1 407.2 100 0\"/><item objectid=\"2\" transform=\"1 0 0 0 1 0 0 0 1 437.2 100 0\"/></build>");
    e.insert("3D/3dmodel.model".into(), model.into_bytes());
    let cfg=String::from_utf8(e["Metadata/model_settings.config"].clone()).unwrap().replace("<assemble>","<plate><metadata key=\"plater_id\" value=\"2\"/><model_instance><metadata key=\"object_id\" value=\"2\"/><metadata key=\"instance_id\" value=\"1\"/></model_instance><model_instance><metadata key=\"object_id\" value=\"2\"/><metadata key=\"instance_id\" value=\"2\"/></model_instance></plate><assemble>");
    e.insert("Metadata/model_settings.config".into(), cfg.into_bytes());
    let audit = transform(&mut e, &catalog().target().unwrap()).unwrap();
    assert_eq!(audit.plates.len(), 2);
    let g = geometry::inspect(&e).unwrap();
    assert!((g.instances[2].transform[9] - g.instances[1].transform[9] - 30.).abs() < 1e-8);
    assert!((audit.plates[1].translation[0] - 4.8).abs() < 1e-8);
}
#[test]
fn oversize_transformed_height_and_cycles_fail() {
    let mut e = fixture();
    let root = String::from_utf8(e["3D/3dmodel.model"].clone())
        .unwrap()
        .replace("0 0 2 10", "0 0 100 10");
    e.insert("3D/3dmodel.model".into(), root.into_bytes());
    assert!(transform(&mut e, &catalog().target().unwrap())
        .unwrap_err()
        .to_string()
        .contains("height"));
    let mut c = catalog();
    c.items
        .get_mut(&("machine".into(), "base".into()))
        .unwrap()
        .insert("inherits".into(), json!(MACHINE));
    assert!(c.target().is_err());
}
#[test]
fn missing_parent_and_unknown_material_rejected() {
    let mut c = catalog();
    c.items.remove(&("machine".into(), "base".into()));
    assert!(c.target().is_err());
    let mut e = fixture();
    let mut p: Value = serde_json::from_slice(&e["Metadata/project_settings.config"]).unwrap();
    p["filament_type"][0] = json!("ABS");
    e.insert(
        "Metadata/project_settings.config".into(),
        serde_json::to_vec(&p).unwrap(),
    );
    assert!(transform(&mut e, &catalog().target().unwrap()).is_err());
}
#[test]
fn layer_instructions_keep_color_pause_remove_custom() {
    let mut e = fixture();
    e.insert("Metadata/custom_gcode_per_layer.xml".into(),br##"<custom_gcodes_per_layer><plate><plate_info id="1"/><mode value="SingleExtruder"/><layer top_z="1" type="0" extruder="2" color="#ff0000" extra=""/><layer top_z="2" type="1" extruder="1" color="" extra="M999"/><layer top_z="3" type="4" extruder="1" color="" extra="M112"/></plate></custom_gcodes_per_layer>"##.to_vec());
    let a = transform(&mut e, &catalog().target().unwrap()).unwrap();
    let s = String::from_utf8(e["Metadata/custom_gcode_per_layer.xml"].clone()).unwrap();
    assert!(!s.contains("M112") && !s.contains("M999"));
    assert!(s.contains("#ff0000") && s.contains("type=\"1\""));
    assert!(a.notes.iter().any(|n| n.contains("Dropped custom")));
}
#[test]
fn reject_bad_variable_layers() {
    let mut e = fixture();
    e.insert(
        "Metadata/layer_heights_profile.txt".into(),
        b"object_id=1|0;0.9".to_vec(),
    );
    assert!(transform(&mut e, &catalog().target().unwrap()).is_err());
}
#[test]
fn recenter_only_when_needed_without_scale() {
    let mut e = fixture();
    let root = String::from_utf8(e["3D/3dmodel.model"].clone())
        .unwrap()
        .replace("100 100 0", "400 100 0");
    e.insert("3D/3dmodel.model".into(), root.into_bytes());
    let before = geometry::inspect(&e).unwrap();
    let a = transform(&mut e, &catalog().target().unwrap()).unwrap();
    assert_ne!(a.plates[0].translation, [0., 0.]);
    let after = geometry::inspect(&e).unwrap();
    for i in 0..3 {
        assert!(
            (before.instances[0].bounds.max[i]
                - before.instances[0].bounds.min[i]
                - (after.instances[0].bounds.max[i] - after.instances[0].bounds.min[i]))
                .abs()
                < 1e-8
        );
    }
}
#[test]
fn reject_exclusions_and_missing_assignments() {
    let mut c = catalog();
    c.items
        .get_mut(&("machine".into(), "base".into()))
        .unwrap()
        .insert(
            "bed_exclude_area".into(),
            json!(["0x0", "260x0", "260x260", "0x260"]),
        );
    assert!(transform(&mut fixture(), &c.target().unwrap()).is_err());
}
#[test]
fn malformed_archives_and_xml_are_rejected() {
    assert!(!archive::safe_name("../bad"));
    assert!(!archive::safe_name("C:/evil"));
    assert!(!archive::safe_name("x\\y"));
    assert!(
        archive::validate_xml(b"<!DOCTYPE a [<!ENTITY x SYSTEM 'file:///x'>]><a>&x;</a>").is_err()
    );
    assert!(archive::validate_xml(b"<a><b></a>").is_err());
    assert!(archive::validate_xml(b"<a/><b/>").is_err());
    assert!(archive::validate_xml(b"<a>&unknown;</a>").is_err());
    assert!(archive::validate_xml(b"<a x='1' x='2'/>").is_err());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.3mf");
    std::fs::write(&path, b"not a zip").unwrap();
    assert!(archive::read(&path).is_err());
    let f = std::fs::File::create(&path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    z.start_file("../escape", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut z, b"bad").unwrap();
    z.finish().unwrap();
    assert!(archive::read(&path).is_err());
}
#[test]
fn zip_roundtrip_and_printer_ready_package_rejection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.3mf");
    let mut f = std::fs::File::create(&path).unwrap();
    let e = fixture();
    archive::write(&mut f, &e).unwrap();
    drop(f);
    assert_eq!(archive::read(&path).unwrap(), e);
    let mut f = std::fs::File::create(&path).unwrap();
    let mut e = Entries::new();
    e.insert("Metadata/plate_1.gcode".into(), b"G28".to_vec());
    archive::write(&mut f, &e).unwrap();
    drop(f);
    assert!(archive::read(&path)
        .unwrap_err()
        .to_string()
        .contains("printer-ready"));
}

#[test]
fn adjacent_unique_outputs_reports_and_original_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("profiles");
    std::fs::create_dir(&root).unwrap();
    for (i, ((category, name), mut settings)) in catalog().items.into_iter().enumerate() {
        settings.insert("type".into(), json!(category));
        settings.insert("name".into(), json!(name));
        std::fs::write(
            root.join(format!("{i}.json")),
            serde_json::to_vec(&settings).unwrap(),
        )
        .unwrap();
    }
    let input = dir.path().join("original.3mf");
    let mut file = std::fs::File::create(&input).unwrap();
    archive::write(&mut file, &fixture()).unwrap();
    drop(file);
    let before = std::fs::read(&input).unwrap();
    let first = kobra_converter::convert(&input, Some(&root), |_| {}).unwrap();
    let second = kobra_converter::convert(&input, Some(&root), |_| {}).unwrap();
    assert_ne!(first.output, second.output);
    assert!(first.output.ends_with("original_KobraX.3mf"));
    assert!(second.output.ends_with("original_KobraX_1.3mf"));
    assert!(std::path::Path::new(&first.report).exists());
    assert_eq!(std::fs::read(&input).unwrap(), before);
    assert_eq!(
        geometry::inspect(&archive::read(std::path::Path::new(&first.output)).unwrap())
            .unwrap()
            .triangles,
        1
    );
}
#[test]
fn nozzle_variants_use_slowest_motion_but_conflicting_quality_rejects() {
    let t = catalog().target().unwrap();
    let mut notes = Vec::new();
    assert_eq!(
        kobra_converter::policy::transfer(
            "outer_wall_speed",
            &json!(["100", "400"]),
            &t,
            &mut notes
        )
        .unwrap(),
        Some(json!("100"))
    );
    assert!(
        kobra_converter::policy::transfer("wall_loops", &json!(["1", "2"]), &t, &mut notes)
            .is_err()
    );
}
#[test]
fn archive_count_size_duplicates_and_dtd_limits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.3mf");
    let mut e = fixture();
    e.insert("DUP.txt".into(), vec![]);
    e.insert("dup.txt".into(), vec![]);
    let mut f = std::fs::File::create(&path).unwrap();
    archive::write(&mut f, &e).unwrap();
    drop(f);
    assert!(archive::read(&path)
        .unwrap_err()
        .to_string()
        .contains("Duplicate"));
    let mut e = fixture();
    e.insert("Metadata/bad.xml".into(), b"<!DOCTYPE a><a/>".to_vec());
    let mut f = std::fs::File::create(&path).unwrap();
    archive::write(&mut f, &e).unwrap();
    drop(f);
    assert!(archive::read(&path).is_err());
    let mut bytes = std::fs::read(&path).unwrap();
    let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
    bytes[central + 24..central + 28]
        .copy_from_slice(&((archive::MAX_ENTRY + 1) as u32).to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    assert!(archive::read(&path).is_err());
    let mut z = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    for i in 0..4097 {
        z.start_file(format!("{i}.txt"), zip::write::SimpleFileOptions::default())
            .unwrap();
    }
    z.finish().unwrap();
    assert!(archive::read(&path)
        .unwrap_err()
        .to_string()
        .contains("Too many"));
}

#[test]
fn relationship_paths_cannot_escape_even_with_uri_encoding() {
    assert_eq!(
        archive::relationship_target("3D/_rels/3dmodel.model.rels", "Objects/part.model").unwrap(),
        "3D/Objects/part.model"
    );
    assert_eq!(
        archive::relationship_target("3D/_rels/3dmodel.model.rels", "../Textures/a%20b.png")
            .unwrap(),
        "Textures/a b.png"
    );
    for target in [
        "../outside",
        "%2e%2e/outside",
        "https://example.invalid/a",
        "C:%5csecret",
        "/a%00b",
    ] {
        assert!(archive::relationship_target("_rels/.rels", target).is_err());
    }
    let mut e = fixture();
    e.insert(
        "_rels/.rels".into(),
        br#"<Relationships><Relationship Target="%2e%2e/outside"/></Relationships>"#.to_vec(),
    );
    assert!(transform(&mut e, &catalog().target().unwrap()).is_err());
}

#[test]
fn native_filament_ids_do_not_abort_import_before_temperatures_and_quality() {
    let mut entries = fixture();
    transform(&mut entries, &catalog().target().unwrap()).unwrap();
    let project: Settings =
        serde_json::from_slice(&entries["Metadata/project_settings.config"]).unwrap();
    kobra_converter::validate_project_metadata(&project, 2).unwrap();
    assert!(!project.contains_key("filament_id"));
    assert_eq!(
        project["filament_ids"],
        json!(["NATIVE_PLA", "NATIVE_PETG"])
    );
    assert_eq!(project["name"], json!("project_settings"));
    assert_eq!(project["print_compatible_printers"], json!([MACHINE]));
    // These sort after filament_id and were previously skipped when Anycubic
    // stopped parsing at the malformed scalar metadata.
    assert_eq!(
        project["filament_settings_id"],
        json!([
            "Native PLA @Anycubic Kobra X 0.4 nozzle",
            "Native PETG @Anycubic Kobra X 0.4 nozzle"
        ])
    );
    assert_eq!(project["nozzle_temperature"], json!(["215", "215"]));
    assert_eq!(project["wall_loops"], json!("1"));
    assert_eq!(project["sparse_infill_density"], json!("5%"));
    let mut malformed = project;
    malformed.insert("filament_id".into(), json!(["BAD_ARRAY"]));
    assert!(kobra_converter::validate_project_metadata(&malformed, 2).is_err());
}

#[test]
fn non_allowlisted_settings_replaced_by_native_defaults_are_reported() {
    let mut c = catalog();
    let process = c
        .items
        .get_mut(&("process".into(), "standard".into()))
        .unwrap();
    process.insert("fuzzy_skin".into(), json!("none"));
    process.insert("precise_outer_wall".into(), json!("0"));
    let mut e = fixture();
    let mut p: Value = serde_json::from_slice(&e["Metadata/project_settings.config"]).unwrap();
    p["fuzzy_skin"] = json!("external");
    p["precise_outer_wall"] = json!("0");
    p["print_settings_id"] = json!("0.20mm Standard @BBL P1S");
    e.insert(
        "Metadata/project_settings.config".into(),
        serde_json::to_vec(&p).unwrap(),
    );
    let audit = transform(&mut e, &c.target().unwrap()).unwrap();
    let out: Value = serde_json::from_slice(&e["Metadata/project_settings.config"]).unwrap();
    assert_eq!(out["fuzzy_skin"], "none");
    assert!(audit
        .notes
        .iter()
        .any(|n| n.starts_with("Dropped project setting fuzzy_skin")));
    // Equal values and rewritten preset identity are not design changes.
    assert!(!audit
        .notes
        .iter()
        .any(|n| n.contains("precise_outer_wall") || n.contains("print_settings_id")));
}

fn with_part(e: &mut Entries, z: &str, subtype: &str) {
    let mesh = String::from_utf8(e["3D/Objects/mesh.model"].clone())
        .unwrap()
        .replace("</resources>", &format!(r#"<object id="3"><mesh><vertices><vertex x="0" y="0" z="{z}"/><vertex x="5" y="0" z="{z}"/><vertex x="0" y="5" z="2"/></vertices><triangles><triangle v1="0" v2="1" v3="2"/></triangles></mesh></object></resources>"#));
    e.insert("3D/Objects/mesh.model".into(), mesh.into_bytes());
    let root = String::from_utf8(e["3D/3dmodel.model"].clone()).unwrap().replace(
        "</components>",
        r#"<component objectid="3" p:path="/3D/Objects/mesh.model"/></components>"#,
    );
    e.insert("3D/3dmodel.model".into(), root.into_bytes());
    let cfg = String::from_utf8(e["Metadata/model_settings.config"].clone())
        .unwrap()
        .replace(
            "</object>",
            &format!(r#"<part id="3" subtype="{subtype}"></part></object>"#),
        );
    e.insert("Metadata/model_settings.config".into(), cfg.into_bytes());
}

#[test]
fn modifier_volumes_outside_the_bed_do_not_affect_fit() {
    for subtype in [
        "modifier_part",
        "negative_part",
        "support_blocker",
        "support_enforcer",
    ] {
        let mut e = fixture();
        with_part(&mut e, "-3", subtype);
        let audit = transform(&mut e, &catalog().target().unwrap()).unwrap();
        assert_eq!(audit.plates[0].translation, [0., 0.], "{subtype}");
        assert_eq!(audit.plates[0].bounds.min[2], 0., "{subtype}");
    }
    let mut e = fixture();
    with_part(&mut e, "-3", "normal_part");
    assert!(transform(&mut e, &catalog().target().unwrap())
        .unwrap_err()
        .to_string()
        .contains("below the bed"));
}

#[test]
fn layer_range_filament_assignments_are_retained_and_checked() {
    let ranges = |extruder: &str| {
        format!(r#"<?xml version="1.0" encoding="utf-8"?><objects><object id="2"><range min_z="2" max_z="5"><option opt_key="extruder">{extruder}</option><option opt_key="layer_height">0.2</option><option opt_key="nozzle_temperature">299</option></range></object></objects>"#).into_bytes()
    };
    let mut e = fixture();
    e.insert("Metadata/layer_config_ranges.xml".into(), ranges("2"));
    transform(&mut e, &catalog().target().unwrap()).unwrap();
    let out = String::from_utf8(e["Metadata/layer_config_ranges.xml"].clone()).unwrap();
    assert!(out.contains(r#"<option opt_key="extruder">2</option>"#));
    assert!(out.contains(r#"<option opt_key="layer_height">0.2</option>"#));
    assert!(!out.contains("299"));
    let mut e = fixture();
    e.insert("Metadata/layer_config_ranges.xml".into(), ranges("9"));
    assert!(transform(&mut e, &catalog().target().unwrap()).is_err());
}
