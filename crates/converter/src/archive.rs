use anyhow::{bail, ensure, Context, Result};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub type Entries = BTreeMap<String, Vec<u8>>;
pub const MAX_ENTRY: u64 = 128 * 1024 * 1024;
pub const MAX_TOTAL: u64 = 512 * 1024 * 1024;

/// Resolve an OPC relationship inside the package, never against the filesystem.
pub fn relationship_target(relationship: &str, target: &str) -> Result<String> {
    let mut decoded = Vec::new();
    let mut input = target.bytes();
    while let Some(b) = input.next() {
        if b == b'%' {
            let hi = input
                .next()
                .and_then(|b| (b as char).to_digit(16))
                .context("Invalid relationship URI escape")?;
            let lo = input
                .next()
                .and_then(|b| (b as char).to_digit(16))
                .context("Invalid relationship URI escape")?;
            decoded.push((hi * 16 + lo) as u8);
        } else {
            decoded.push(b);
        }
    }
    let target = std::str::from_utf8(&decoded)?;
    ensure!(
        !target.contains(['\\', ':', '\0', '?', '#']),
        "External or unsafe package relationship"
    );
    let mut parts = if target.starts_with('/') || relationship == "_rels/.rels" {
        Vec::new()
    } else {
        relationship
            .split_once("/_rels/")
            .context("Unsupported relationship location")?
            .0
            .split('/')
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                ensure!(
                    parts.pop().is_some(),
                    "Package relationship escapes the archive"
                );
            }
            p => parts.push(p.to_owned()),
        }
    }
    let name = parts.join("/");
    ensure!(safe_name(&name), "Unsafe package relationship");
    Ok(name)
}

pub fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains(['\\', ':', '\0'])
        && name.split('/').all(|p| !matches!(p, ".." | "."))
}

pub fn read(path: &Path) -> Result<Entries> {
    let file = File::open(path).context("Could not open the selected file")?;
    ensure!(
        file.metadata()?.len() <= MAX_TOTAL,
        "The archive exceeds the 512 MiB input limit"
    );
    let mut zip = ZipArchive::new(file).context("This is not a readable 3MF archive")?;
    ensure!(zip.len() <= 4096, "Too many archive entries (limit 4096)");
    let mut total = 0;
    let mut names = std::collections::HashSet::new();
    let mut out = Entries::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name()?.to_string();
        ensure!(safe_name(&name), "Unsafe archive path: {name}");
        ensure!(
            names.insert(name.to_lowercase()),
            "Duplicate archive path: {name}"
        );
        ensure!(!entry.encrypted(), "Encrypted archives are not supported");
        ensure!(
            entry
                .unix_mode()
                .map(|m| m & 0o170000 != 0o120000)
                .unwrap_or(true),
            "Archive links are not supported"
        );
        ensure!(
            entry.size() <= MAX_ENTRY,
            "An archive entry exceeds 128 MiB: {name}"
        );
        total += entry.size();
        ensure!(total <= MAX_TOTAL, "Expanded archive exceeds 512 MiB");
        ensure!(
            entry.size() <= entry.compressed_size().max(1).saturating_mul(2000),
            "Suspicious compression ratio: {name}"
        );
        if entry.is_dir() {
            continue;
        }
        let mut bytes = Vec::new();
        (&mut entry).take(MAX_ENTRY + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_ENTRY,
            "Archive size limit exceeded"
        );
        // Reading to EOF verifies ZIP CRC as well as the central-directory sizes.
        ensure!(
            bytes.len() as u64 == entry.size(),
            "Inconsistent archive entry size"
        );
        out.insert(name, bytes);
    }
    let has_model = out.keys().any(|k| k.to_lowercase().ends_with(".model"));
    if !has_model && out.keys().any(|k| k.to_lowercase().ends_with("gcode")) {
        bail!("This is a printer-ready G-code package. Download the editable 3MF project instead.");
    }
    ensure!(
        out.contains_key("3D/3dmodel.model"),
        "No supported editable 3MF model was found"
    );
    for (name, bytes) in &out {
        if name.ends_with(".model")
            || name.ends_with(".xml")
            || name.ends_with(".rels")
            || (name.ends_with(".config") && bytes.first() == Some(&b'<'))
        {
            validate_xml(bytes).with_context(|| format!("Invalid XML in {name}"))?;
        }
    }
    Ok(out)
}

pub fn validate_xml(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).context("Only UTF-8 XML is supported")?;
    let lower = text.to_ascii_lowercase();
    ensure!(
        !lower.contains("<!doctype") && !lower.contains("<!entity"),
        "XML entities and DTDs are disabled"
    );
    let mut reader = quick_xml::Reader::from_str(text);
    let mut depth = 0usize;
    let mut nodes = 0usize;
    let mut roots = 0usize;
    loop {
        match reader.read_event()? {
            quick_xml::events::Event::Start(e) => {
                if depth == 0 {
                    roots += 1;
                }
                for a in e.attributes() {
                    a?.normalized_value(quick_xml::XmlVersion::Implicit1_0)?;
                }
                depth += 1;
                nodes += 1;
            }
            quick_xml::events::Event::Empty(e) => {
                if depth == 0 {
                    roots += 1;
                }
                for a in e.attributes() {
                    a?.normalized_value(quick_xml::XmlVersion::Implicit1_0)?;
                }
                nodes += 1;
            }
            quick_xml::events::Event::GeneralRef(e) => {
                let name = e.as_ref();
                ensure!(
                    matches!(name, "amp" | "lt" | "gt" | "quot" | "apos")
                        || e.resolve_char_ref()?.is_some(),
                    "Unknown XML entity"
                );
            }
            quick_xml::events::Event::End(_) => {
                ensure!(depth > 0, "Unbalanced XML");
                depth -= 1;
            }
            quick_xml::events::Event::DocType(_) => bail!("DTD is disabled"),
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
        ensure!(
            depth <= 64 && nodes <= 4_000_000,
            "XML complexity limit exceeded"
        );
    }
    ensure!(
        depth == 0 && roots == 1,
        "Truncated XML or multiple document roots"
    );
    Ok(())
}

pub fn write(file: &mut File, entries: &Entries) -> Result<()> {
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        zip.start_file(name, options)?;
        zip.write_all(bytes)?;
    }
    zip.finish()?.sync_all()?;
    Ok(())
}
