use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

pub type Settings = Map<String, Value>;
pub const MACHINE: &str = "Anycubic Kobra X 0.4 nozzle";

pub fn scalar(v: &Value) -> &Value {
    if let Some(a) = v.as_array() {
        if a.len() == 1 {
            return scalar(&a[0]);
        }
    }
    v
}
pub fn string(v: &Value) -> String {
    scalar(v)
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| scalar(v).to_string())
}
pub fn number(v: &Value) -> Result<f64> {
    let n: f64 = string(v)
        .parse()
        .context("Expected a finite numeric preset value")?;
    ensure!(n.is_finite(), "Non-finite preset value");
    Ok(n)
}
pub fn num(s: &Settings, k: &str) -> Result<f64> {
    number(
        s.get(k)
            .with_context(|| format!("Target preset is missing {k}"))?,
    )
}

#[derive(Default)]
pub struct Catalog {
    pub items: BTreeMap<(String, String), Settings>,
}
impl Catalog {
    pub fn load(roots: &[PathBuf]) -> Result<Self> {
        fn walk(c: &mut Catalog, p: &Path, depth: usize, count: &mut usize) -> Result<()> {
            if depth > 8 || !p.exists() {
                return Ok(());
            }
            for e in std::fs::read_dir(p)? {
                let e = e?;
                let kind = e.file_type()?;
                if kind.is_symlink() {
                    continue;
                }
                if kind.is_dir() {
                    walk(c, &e.path(), depth + 1, count)?;
                } else if e.path().extension().and_then(|x| x.to_str()) == Some("json") {
                    *count += 1;
                    ensure!(*count <= 30000, "Too many installed profile files");
                    if e.metadata()?.len() > 2 * 1024 * 1024 {
                        continue;
                    }
                    let bytes = std::fs::read(e.path())?;
                    let text = std::str::from_utf8(&bytes)?.trim_start_matches('\u{feff}');
                    let Ok(s) = serde_json::from_str::<Settings>(text) else {
                        continue;
                    };
                    let category = s
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let name = s
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            e.path().file_stem().unwrap().to_string_lossy().into_owned()
                        });
                    if matches!(category.as_str(), "machine" | "process" | "filament") {
                        c.items.insert((category, name), s);
                    }
                }
            }
            Ok(())
        }
        let mut c = Self::default();
        let mut count = 0;
        for root in roots {
            walk(&mut c, root, 0, &mut count)?;
        }
        Ok(c)
    }
    pub fn resolve(&self, category: &str, name: &str) -> Result<Settings> {
        fn inner(
            c: &Catalog,
            cat: &str,
            name: &str,
            seen: &mut HashSet<String>,
        ) -> Result<Settings> {
            ensure!(
                seen.len() < 32 && seen.insert(name.to_string()),
                "Profile inheritance cycle or excessive depth: {name}"
            );
            let s = c.items.get(&(cat.to_string(), name.to_string())).with_context(|| format!("Missing installed {cat} profile: {name}. Locate the slicer's profiles folder."))?;
            let mut out = if let Some(parent) = s
                .get("inherits")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                inner(c, cat, parent, seen)?
            } else {
                Settings::new()
            };
            out.extend(s.clone());
            Ok(out)
        }
        inner(self, category, name, &mut HashSet::new())
    }
    pub fn target(&self) -> Result<Target> {
        let machine = self.resolve("machine", MACHINE)?;
        ensure!(
            (num(&machine, "nozzle_diameter")? - 0.4).abs() < 1e-6,
            "The installed Kobra X profile is not for a 0.4 mm nozzle"
        );
        ensure!(
            machine.get("printer_model").and_then(Value::as_str) == Some("Anycubic Kobra X"),
            "Unsupported target printer model"
        );
        let process_name = machine
            .get("default_print_profile")
            .map(string)
            .context("Machine has no native default process")?;
        let process = self.resolve("process", &process_name)?;
        let mut filaments = BTreeMap::new();
        for ((cat, name), _) in &self.items {
            if cat != "filament" {
                continue;
            }
            if !name.contains("@Anycubic Kobra X 0.4 nozzle") {
                continue;
            }
            let f = self.resolve("filament", name)?;
            if let Some(t) = f.get("filament_type") {
                filaments
                    .entry(string(t).to_uppercase())
                    .or_insert((name.clone(), f));
            }
        }
        ensure!(
            !filaments.is_empty(),
            "No native Kobra X 0.4 mm filament profiles found"
        );
        Ok(Target {
            machine,
            process,
            process_name,
            filaments,
        })
    }
}
pub struct Target {
    pub machine: Settings,
    pub process: Settings,
    pub process_name: String,
    pub filaments: BTreeMap<String, (String, Settings)>,
}
impl Target {
    pub fn material(&self, material: &str) -> Result<&(String, Settings)> {
        self.filaments.get(&material.to_uppercase()).with_context(|| format!("No native Kobra X default for material '{material}'. Select a supported material in the source project first; Kobra Drop will not assume PLA."))
    }
}

pub fn discover(extra: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(p) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(p).join("AnycubicSlicerNext/resources/profiles"));
    }
    if let Some(p) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(p).join("AnycubicSlicerNext/system"));
    }
    if let Some(p) = extra {
        roots.push(p.into());
    }
    roots
}

#[derive(Serialize, Deserialize)]
pub struct Discovery {
    pub found: bool,
    pub message: String,
    pub materials: Vec<String>,
}
pub fn status(extra: Option<&Path>) -> Discovery {
    match Catalog::load(&discover(extra)).and_then(|c| c.target()) {
        Ok(t) => Discovery {
            found: true,
            message: format!("{MACHINE} · installed native profiles"),
            materials: t.filaments.keys().cloned().collect(),
        },
        Err(e) => Discovery {
            found: false,
            message: format!("{e:#}"),
            materials: vec![],
        },
    }
}

pub fn merge_native(out: &mut Settings, source: &Settings) {
    for (k, v) in source {
        if !matches!(
            k.as_str(),
            "name"
                | "type"
                | "inherits"
                | "from"
                | "instantiation"
                | "setting_id"
                | "version"
                | "is_custom_defined"
                | "compatible_printers"
                | "compatible_printers_condition"
                | "compatible_prints"
                | "compatible_prints_condition"
        ) && !k.starts_with("printhost")
        {
            out.insert(k.clone(), v.clone());
        }
    }
}

pub fn polygon(s: &Settings, key: &str) -> Result<Vec<[f64; 2]>> {
    let Some(a) = s.get(key).and_then(Value::as_array) else {
        if key == "bed_exclude_area" {
            return Ok(vec![]);
        }
        bail!("Target profile lacks printable bed polygon");
    };
    a.iter()
        .map(|v| {
            let text = v.as_str().context("Invalid bed coordinate")?;
            let (x, y) = text.split_once('x').context("Invalid bed coordinate")?;
            let (x, y): (f64, f64) = (x.parse()?, y.parse()?);
            ensure!(x.is_finite() && y.is_finite(), "Invalid bed coordinate");
            Ok([x, y])
        })
        .collect()
}
