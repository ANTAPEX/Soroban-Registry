use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::io_utils::{compute_sha256_streaming, extract_tar_gz};
use crate::manifest::{AuditEntry, ExportManifest};

pub fn extract_and_verify(archive_path: &Path, output_dir: &Path) -> Result<ExportManifest> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;

    extract_tar_gz(archive_path, tmp_dir.path())?;

    let manifest_path = tmp_dir.path().join("manifest.json");
    let inner_path = tmp_dir.path().join("contract.tar.gz");

    if !manifest_path.exists() || !inner_path.exists() {
        bail!("invalid archive: missing manifest.json or contract.tar.gz");
    }

    let mut manifest: ExportManifest =
        serde_json::from_reader(BufReader::new(File::open(&manifest_path)?))?;

    let computed_hash = compute_sha256_streaming(&inner_path)?;
    if computed_hash != manifest.sha256 {
        bail!(
            "integrity check failed: expected {} got {}",
            manifest.sha256,
            computed_hash
        );
    }

    manifest.audit_trail.push(AuditEntry {
        action: "import_verified".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    fs::create_dir_all(output_dir)?;
    extract_tar_gz(&inner_path, output_dir)?;

    manifest.audit_trail.push(AuditEntry {
        action: "import_extracted".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    Ok(manifest)
}
