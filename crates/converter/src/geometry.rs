use crate::{
    archive::Entries,
    profiles::{num, polygon, Settings},
};
use anyhow::{ensure, Context, Result};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

pub type Matrix = [f64; 12];
pub const IDENTITY: Matrix = [1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.];
fn attr(e: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>> {
    for a in e.attributes() {
        let a = a?;
        if a.key.local_name().as_ref() == std::str::from_utf8(key)? {
            return Ok(Some(
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}
fn required(e: &BytesStart<'_>, key: &[u8]) -> Result<String> {
    attr(e, key)?.with_context(|| {
        format!(
            "Missing geometry attribute {}",
            String::from_utf8_lossy(key)
        )
    })
}
pub fn matrix(s: Option<&str>) -> Result<Matrix> {
    let Some(s) = s else {
        return Ok(IDENTITY);
    };
    let v = s
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        v.len() == 12 && v.iter().all(|x| x.is_finite() && x.abs() < 1e8),
        "Invalid component or instance transform"
    );
    let m: Matrix = v.try_into().unwrap();
    let det = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]);
    ensure!(det.abs() > 1e-12, "A transform collapses model dimensions");
    Ok(m)
}
pub fn point(p: [f64; 3], m: &Matrix) -> [f64; 3] {
    [
        p[0] * m[0] + p[1] * m[3] + p[2] * m[6] + m[9],
        p[0] * m[1] + p[1] * m[4] + p[2] * m[7] + m[10],
        p[0] * m[2] + p[1] * m[5] + p[2] * m[8] + m[11],
    ]
}
fn compose(child: &Matrix, parent: &Matrix) -> Matrix {
    let mut out = [0.; 12];
    for i in 0..3 {
        for j in 0..3 {
            out[i * 3 + j] = (0..3).map(|k| child[i * 3 + k] * parent[k * 3 + j]).sum();
        }
    }
    out[9..12].copy_from_slice(&point([child[9], child[10], child[11]], parent));
    out
}
#[derive(Default)]
struct Object {
    vertices: Vec<[f64; 3]>,
    components: Vec<(String, String, Matrix)>,
    triangles: usize,
    max_index: usize,
}
#[derive(Clone)]
pub struct Instance {
    pub id: String,
    pub index: usize,
    pub transform: Matrix,
    pub bounds: Bounds,
}
#[derive(Clone, Copy, Serialize, Debug)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        }
    }
}
impl Bounds {
    fn add(&mut self, p: [f64; 3]) {
        for (i, n) in p.iter().enumerate() {
            self.min[i] = self.min[i].min(*n);
            self.max[i] = self.max[i].max(*n);
        }
    }
    fn merge(&mut self, b: Self) {
        self.add(b.min);
        self.add(b.max);
    }
    fn shifted(&self, d: [f64; 2]) -> Self {
        let mut b = *self;
        for (i, delta) in d.iter().enumerate() {
            b.min[i] += delta;
            b.max[i] += delta;
        }
        b
    }
}
pub struct Geometry {
    pub instances: Vec<Instance>,
    pub vertices: usize,
    pub triangles: usize,
}

pub fn inspect(entries: &Entries) -> Result<Geometry> {
    let mut objects = HashMap::<(String, String), Object>::new();
    let mut builds = Vec::new();
    let mut count = 0;
    let mut tris = 0;
    for (file, bytes) in entries.iter().filter(|(n, _)| n.ends_with(".model")) {
        let mut reader = Reader::from_reader(bytes.as_slice());
        let mut current: Option<(String, Object)> = None;
        let mut instances = HashMap::<String, usize>::new();
        loop {
            match reader.read_event()? {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    match e.local_name().as_ref() {
                        "model" => {
                            ensure!(
                                attr(e, b"unit")?.as_deref().unwrap_or("millimeter")
                                    == "millimeter",
                                "Only millimeter 3MF projects are supported"
                            );
                            if let Some(required) = attr(e, b"requiredextensions")? {
                                // Production components and Bambu painting are retained; other
                                // required extensions need a dedicated compatibility implementation.
                                for prefix in required.split_whitespace() {
                                    ensure!(
                                        matches!(prefix, "p" | "BambuStudio"),
                                        "Unsupported required 3MF extension: {prefix}"
                                    );
                                }
                            }
                        }
                        "object" => {
                            ensure!(current.is_none(), "Nested geometry objects");
                            current = Some((required(e, b"id")?, Object::default()));
                        }
                        "vertex" => {
                            let ob = &mut current.as_mut().context("Vertex outside object")?.1;
                            let mut p = [0.0f64; 3];
                            for (i, k) in [b"x", b"y", b"z"].iter().enumerate() {
                                p[i] = required(e, *k)?.parse()?;
                            }
                            ensure!(
                                p.iter().all(|x| x.is_finite() && x.abs() < 1e7),
                                "Invalid mesh coordinate"
                            );
                            ob.vertices.push(p);
                            count += 1;
                            ensure!(count <= 3_000_000, "Mesh vertex limit exceeded");
                        }
                        "triangle" => {
                            let ob = &mut current.as_mut().context("Triangle outside object")?.1;
                            for k in [b"v1", b"v2", b"v3"] {
                                ob.max_index = ob.max_index.max(required(e, k)?.parse()?);
                            }
                            ob.triangles += 1;
                            tris += 1;
                            ensure!(tris <= 3_000_000, "Mesh triangle limit exceeded");
                            for a in e.attributes() {
                                let a = a?;
                                let key = a.key.local_name();
                                let key = key.as_ref();
                                ensure!(
                                    !key.to_lowercase().contains("gcode"),
                                    "Embedded triangle instructions are unsupported"
                                );
                            }
                        }
                        "component" => {
                            let ob = &mut current.as_mut().context("Component outside object")?.1;
                            let raw = attr(e, b"path")?.unwrap_or_else(|| file.clone());
                            let component_file = if raw.starts_with('/') {
                                raw.trim_start_matches('/').to_string()
                            } else if raw == *file {
                                raw
                            } else {
                                format!(
                                    "{}/{}",
                                    file.rsplit_once('/').map(|p| p.0).unwrap_or(""),
                                    raw
                                )
                            };
                            ensure!(
                                crate::archive::safe_name(&component_file),
                                "Unsafe component reference"
                            );
                            ob.components.push((
                                component_file,
                                required(e, b"objectid")?,
                                matrix(attr(e, b"transform")?.as_deref())?,
                            ));
                        }
                        "item" if file == "3D/3dmodel.model" => {
                            let id = required(e, b"objectid")?;
                            let index = *instances.entry(id.clone()).or_insert(0);
                            *instances.get_mut(&id).unwrap() += 1;
                            builds.push((id, index, matrix(attr(e, b"transform")?.as_deref())?));
                        }
                        "metadata" => {
                            let name = attr(e, b"name")?.unwrap_or_default().to_lowercase();
                            ensure!(!(name.contains("gcode")||name.contains("slic3rpe:config")||name.contains("slic3r:config")),"Embedded geometry configuration or G-code is unsupported; save as a Bambu/Orca project first");
                        }
                        _ => {}
                    }
                }
                Event::End(e) if e.local_name().as_ref() == "object" => {
                    let (id, ob) = current.take().context("Unbalanced geometry object")?;
                    ensure!(
                        ob.triangles == 0 || ob.max_index < ob.vertices.len(),
                        "Triangle references a missing vertex"
                    );
                    ensure!(
                        objects.insert((file.clone(), id), ob).is_none(),
                        "Duplicate geometry object ID"
                    );
                    ensure!(objects.len() <= 20000, "Object count limit exceeded");
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }
    // Modifiers, negative parts and support blockers/enforcers are not printed,
    // so the slicer ignores them for bed placement. They are still validated.
    let mut helpers = HashSet::<(String, String)>::new();
    if let Some(config) = entries.get("Metadata/model_settings.config") {
        let doc = roxmltree::Document::parse(std::str::from_utf8(config)?)?;
        for object in doc.descendants().filter(|n| n.has_tag_name("object")) {
            let Some(id) = object.attribute("id") else {
                continue;
            };
            for part in object.children().filter(|n| n.has_tag_name("part")) {
                let helper = matches!(
                    part.attribute("subtype"),
                    Some(
                        "modifier_part" | "negative_part" | "support_blocker" | "support_enforcer"
                    )
                ) || matches!(
                    meta(part, "volume_type"),
                    Some(
                        "ModifierVolume" | "NegativeVolume" | "SupportBlocker" | "SupportEnforcer"
                    )
                );
                if let (true, Some(part)) = (helper, part.attribute("id")) {
                    helpers.insert((id.to_string(), part.to_string()));
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn expand(
        key: &(String, String),
        m: &Matrix,
        obs: &HashMap<(String, String), Object>,
        helpers: &HashSet<(String, String)>,
        printed: bool,
        active: &mut HashSet<(String, String)>,
        budget: &mut usize,
        bounds: &mut Bounds,
    ) -> Result<()> {
        ensure!(
            active.len() < 64 && active.insert(key.clone()),
            "Component reference cycle or excessive depth"
        );
        let o = obs
            .get(key)
            .context("Component references a missing model or object")?;
        for v in &o.vertices {
            *budget += 1;
            ensure!(
                *budget <= 20_000_000,
                "Expanded component complexity limit exceeded"
            );
            let p = point(*v, m);
            ensure!(
                p.iter().all(|x| x.is_finite() && x.abs() < 1e8),
                "Transformed coordinate exceeds limits"
            );
            if printed {
                bounds.add(p);
            }
        }
        for (file, id, child) in &o.components {
            // model_settings part IDs name the root object's component object IDs.
            let root = key.0 == "3D/3dmodel.model";
            expand(
                &(file.clone(), id.clone()),
                &compose(child, m),
                obs,
                helpers,
                printed && !(root && helpers.contains(&(key.1.clone(), id.clone()))),
                active,
                budget,
                bounds,
            )?;
        }
        active.remove(key);
        Ok(())
    }
    ensure!(
        !builds.is_empty() && builds.len() <= 10000,
        "No build items, or too many instances"
    );
    let mut result = Vec::new();
    let mut budget = 0;
    for (id, index, transform) in builds {
        let mut bounds = Bounds::default();
        expand(
            &("3D/3dmodel.model".into(), id.clone()),
            &transform,
            &objects,
            &helpers,
            true,
            &mut HashSet::new(),
            &mut budget,
            &mut bounds,
        )?;
        ensure!(
            bounds.min.iter().all(|x| x.is_finite()),
            "Build item has no supported mesh geometry"
        );
        result.push(Instance {
            id,
            index,
            transform,
            bounds,
        });
    }
    Ok(Geometry {
        instances: result,
        vertices: count,
        triangles: tris,
    })
}

#[derive(Serialize, Clone, Debug)]
pub struct PlateResult {
    pub plate: usize,
    pub bounds: Bounds,
    pub translation: [f64; 2],
}
fn meta<'a>(node: roxmltree::Node<'a, 'a>, key: &str) -> Option<&'a str> {
    node.children()
        .find(|n| n.has_tag_name("metadata") && n.attribute("key") == Some(key))
        .and_then(|n| n.attribute("value"))
}
fn rectangle(poly: &[[f64; 2]]) -> Result<[f64; 4]> {
    ensure!(
        poly.len() == 4,
        "Only rectangular native print beds are supported in this version"
    );
    let b = [
        poly.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
        poly.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
        poly.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max),
        poly.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max),
    ];
    ensure!(
        poly.iter()
            .all(|p| (p[0] == b[0] || p[0] == b[2]) && (p[1] == b[1] || p[1] == b[3])),
        "Unsupported bed polygon"
    );
    Ok(b)
}
pub fn place(
    entries: &mut Entries,
    geo: &Geometry,
    source: &Settings,
    target: &Settings,
) -> Result<Vec<PlateResult>> {
    let config = std::str::from_utf8(
        entries
            .get("Metadata/model_settings.config")
            .context("Missing model settings and plate assignments")?,
    )?;
    let doc = roxmltree::Document::parse(config)?;
    let plates = doc
        .descendants()
        .filter(|n| n.has_tag_name("plate"))
        .collect::<Vec<_>>();
    ensure!(plates.len() <= 100, "Too many plates");
    let plate_count = plates.len().max(1);
    let cols = (plate_count as f64).sqrt().ceil() as usize;
    let mut assignments = HashMap::<(String, usize), usize>::new();
    for (i, p) in plates.iter().enumerate() {
        if let Some(id) = meta(*p, "plater_id") {
            ensure!(id.parse::<usize>()? == i + 1, "Unsupported plate numbering");
        }
        for inst in p.children().filter(|n| n.has_tag_name("model_instance")) {
            let id = meta(inst, "object_id")
                .context("Plate instance missing object ID")?
                .to_string();
            let index = meta(inst, "instance_id")
                .context("Plate instance missing index")?
                .parse()?;
            ensure!(
                assignments.insert((id, index), i).is_none(),
                "Instance assigned to multiple plates"
            );
        }
    }
    let bed = rectangle(&polygon(target, "printable_area")?)?;
    let old = rectangle(&polygon(source, "printable_area")?)?;
    let (width, depth) = (bed[2] - bed[0], bed[3] - bed[1]);
    let exclusions = polygon(target, "bed_exclude_area")?;
    ensure!(
        exclusions.len() % 4 == 0,
        "Unsupported bed exclusion polygon"
    );
    let ex = exclusions
        .chunks(4)
        .map(rectangle)
        .collect::<Result<Vec<_>>>()?;
    let height = num(target, "printable_height")?;
    let mut grouped = BTreeMap::<usize, Vec<usize>>::new();
    for (n, inst) in geo.instances.iter().enumerate() {
        let plate = assignments
            .get(&(inst.id.clone(), inst.index))
            .copied()
            .or_else(|| {
                if plate_count == 1 && assignments.is_empty() {
                    Some(0)
                } else {
                    None
                }
            })
            .context(
                "An instance has no plate assignment; arrange it in the source slicer first",
            )?;
        grouped.entry(plate).or_default().push(n);
    }
    ensure!(
        assignments
            .keys()
            .all(|(id, i)| geo.instances.iter().any(|x| x.id == *id && x.index == *i)),
        "Plate references missing build instance"
    );
    let mut shifts = HashMap::<usize, [f64; 2]>::new();
    let mut reports = Vec::new();
    for (plate, instances) in grouped {
        let mut bounds = Bounds::default();
        for n in &instances {
            bounds.merge(geo.instances[*n].bounds);
        }
        let src_origin = [
            (plate % cols) as f64 * (old[2] - old[0]) * 1.2,
            -((plate / cols) as f64) * (old[3] - old[1]) * 1.2,
        ];
        let origin = [
            (plate % cols) as f64 * width * 1.2,
            -((plate / cols) as f64) * depth * 1.2,
        ];
        let local = bounds.shifted([-src_origin[0], -src_origin[1]]);
        ensure!(
            local.min[2] >= -0.05 && local.max[2] <= height + 1e-5,
            "Plate {} exceeds the target height or extends below the bed; geometry was not changed",
            plate + 1
        );
        ensure!(
            local.max[0] - local.min[0] <= width + 1e-5
                && local.max[1] - local.min[1] <= depth + 1e-5,
            "Plate {} does not fit the Kobra X bed; scaling is never automatic",
            plate + 1
        );
        let fits = |b: Bounds| {
            b.min[0] >= bed[0] - 1e-5
                && b.min[1] >= bed[1] - 1e-5
                && b.max[0] <= bed[2] + 1e-5
                && b.max[1] <= bed[3] + 1e-5
                && ex.iter().all(|e| {
                    b.max[0] <= e[0] || b.min[0] >= e[2] || b.max[1] <= e[1] || b.min[1] >= e[3]
                })
        };
        let mut delta = [0.; 2];
        if !fits(local) {
            delta = [
                (bed[0] + bed[2] - local.min[0] - local.max[0]) / 2.,
                (bed[1] + bed[3] - local.min[1] - local.max[1]) / 2.,
            ];
        }
        let placed = local.shifted(delta);
        ensure!(
            fits(placed),
            "Plate {} intersects a bed exclusion; arrange the objects in the source slicer",
            plate + 1
        );
        let shift = [
            origin[0] - src_origin[0] + delta[0],
            origin[1] - src_origin[1] + delta[1],
        ];
        for n in instances {
            shifts.insert(n, shift);
        }
        reports.push(PlateResult {
            plate: plate + 1,
            bounds: placed,
            translation: shift,
        });
    }
    if shifts
        .values()
        .all(|d| d[0].abs() < 1e-9 && d[1].abs() < 1e-9)
    {
        return Ok(reports);
    }
    let model = std::str::from_utf8(entries.get("3D/3dmodel.model").unwrap())?;
    ensure!(
        model.len() < 4 * 1024 * 1024,
        "Root model too large to recenter safely in this version"
    );
    let doc = roxmltree::Document::parse(model)?;
    let mut edits = Vec::new();
    for (i, node) in doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("item") && n.parent().map(|p| p.has_tag_name("build")).unwrap_or(false)
        })
        .enumerate()
    {
        let shift = shifts.get(&i).context("Missing plate translation")?;
        let mut m = geo.instances[i].transform;
        m[9] += shift[0];
        m[10] += shift[1];
        let transform = m.iter().map(f64::to_string).collect::<Vec<_>>().join(" ");
        let range = node.range();
        let fragment = &model[range.clone()];
        let replacement = if let Some(a) = node.attribute_node("transform") {
            let r = a.range_value();
            let mut frag = fragment.to_string();
            frag.replace_range(r.start - range.start..r.end - range.start, &transform);
            frag
        } else {
            let pos = fragment.find('>').context("Invalid item XML")?;
            let insert = if fragment[..pos].ends_with('/') {
                pos - 1
            } else {
                pos
            };
            let mut frag = fragment.to_string();
            frag.insert_str(insert, &format!(" transform=\"{transform}\""));
            frag
        };
        edits.push((range, replacement));
    }
    let mut out = model.to_string();
    for (range, s) in edits.into_iter().rev() {
        out.replace_range(range, &s);
    }
    entries.insert("3D/3dmodel.model".into(), out.into_bytes());
    Ok(reports)
}
