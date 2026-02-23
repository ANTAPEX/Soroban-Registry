use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;

use crate::io_utils::{compute_sha256_streaming, BUF_SIZE};
use crate::manifest::{ExportManifest, ManifestEntry};

pub fn create_archive(
    contract_dir: &Path,
    output_path: &Path,
    contract_id: &str,
    name: &str,
    network: &str,
) -> Result<()> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;
    let inner_path = tmp_dir.path().join("contract.tar.gz");

    let mut manifest = ExportManifest::new(contract_id.into(), name.into(), network.into());

    build_inner_archive(contract_dir, &inner_path, &mut manifest)?;
    manifest.sha256 = compute_sha256_streaming(&inner_path)?;

    let manifest_path = tmp_dir.path().join("manifest.json");
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(&manifest_path, &manifest_json)?;

    build_outer_archive(output_path, &manifest_path, &inner_path)?;

    Ok(())
}

fn build_inner_archive(
    source_dir: &Path,
    archive_path: &Path,
    manifest: &mut ExportManifest,
) -> Result<()> {
    let file = BufWriter::new(File::create(archive_path)?);
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    walk_and_append(&mut builder, source_dir, source_dir, manifest)?;

    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn walk_and_append<W: Write>(
    builder: &mut Builder<W>,
    base: &Path,
    dir: &Path,
    manifest: &mut ExportManifest,
) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);

        if path.is_dir() {
            walk_and_append(builder, base, &path, manifest)?;
        } else {
            let metadata = entry.metadata()?;
            let modified: DateTime<Utc> = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).ok().and_then(|d| {
                        Utc.timestamp_opt(d.as_secs() as i64, d.subsec_nanos())
                            .single()
                    })
                })
                .unwrap_or_else(Utc::now);

            manifest.contents.push(ManifestEntry {
                path: rel.to_string_lossy().replace('\\', "/"),
                size: metadata.len(),
                modified_at: modified,
            });

            let mut header = tar::Header::new_gnu();
            header.set_size(metadata.len());
            header.set_mode(0o644);
            header.set_cksum();

            let f = BufReader::new(File::open(&path)?);
            builder.append_data(&mut header, rel.to_string_lossy().replace('\\', "/"), f)?;
        }
    }
    Ok(())
}

fn build_outer_archive(
    output_path: &Path,
    manifest_path: &Path,
    inner_archive_path: &Path,
) -> Result<()> {
    let file = BufWriter::new(File::create(output_path)?);
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    append_file_streaming(&mut builder, manifest_path, "manifest.json")?;
    append_file_streaming(&mut builder, inner_archive_path, "contract.tar.gz")?;

    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn append_file_streaming<W: Write>(
    builder: &mut Builder<W>,
    file_path: &Path,
    archive_name: &str,
) -> Result<()> {
    let metadata = fs::metadata(file_path)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(metadata.len());
    header.set_mode(0o644);
    header.set_cksum();

    let reader = BufReader::with_capacity(BUF_SIZE, File::open(file_path)?);
    builder.append_data(&mut header, archive_name, reader)?;
    Ok(())
}
