use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::Result;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;

pub const BUF_SIZE: usize = 65536;

/// Compute SHA256 hash of a file using streaming to handle large files.
pub fn compute_sha256_streaming(path: &Path) -> Result<String> {
    let mut reader = BufReader::with_capacity(BUF_SIZE, File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; BUF_SIZE];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Extract a gzipped tar archive to a destination directory.
pub fn extract_tar_gz(archive_path: &Path, dest: &Path) -> Result<()> {
    let reader = BufReader::with_capacity(BUF_SIZE, File::open(archive_path)?);
    let decoder = GzDecoder::new(reader);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let dest_path = dest.join(&path);

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = BufWriter::new(File::create(&dest_path)?);
        let mut buf = vec![0u8; BUF_SIZE];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
        }
        out.flush()?;
    }

    Ok(())
}
