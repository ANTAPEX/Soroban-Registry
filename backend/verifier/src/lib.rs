// Contract Verification Engine
//
// Compiles Soroban contract source code and verifies it matches on-chain bytecode.
// Accepts source via Git URL or direct upload, compiles with the Soroban/Rust
// toolchain, hashes the resulting WASM, and compares against the stored wasm_hash.
//
// Timeout: 5 minutes per verification attempt.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::process::Command;
use tokio::time::timeout;

use shared::RegistryError;

/// Default Soroban SDK version used when none is specified.
pub const DEFAULT_SOROBAN_SDK_VERSION: &str = "21.7.7";

/// Maximum time allowed for a single verification run (5 minutes).
const VERIFICATION_TIMEOUT: Duration = Duration::from_secs(300);

// ─── Public Types ────────────────────────────────────────────────────────────

/// How the source code is supplied to the verification engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceInput {
    /// Clone from a public Git repository.
    GitUrl {
        url: String,
        /// Branch to check out. Defaults to the repo's default branch.
        branch: Option<String>,
        /// Pin to an exact commit SHA (checked out after clone).
        commit: Option<String>,
    },
    /// Raw Rust source provided directly (single-file Soroban contract or a
    /// special `wasm_base64:<base64>` payload for pre-compiled test artefacts).
    SourceCode { code: String },
}

/// Everything the engine needs to perform one verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub source: SourceInput,
    /// Soroban SDK / toolchain version (e.g. "21.7.7").
    pub compiler_version: Option<String>,
    /// Arbitrary extra build parameters (supports `profile` and `features` keys).
    pub build_params: Option<Value>,
    /// The SHA-256 hex digest stored on-chain / in the contracts table.
    pub expected_wasm_hash: String,
}

/// The final verdict of one verification attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationOutcome {
    Verified,
    Failed { reason: String },
}

/// Full result returned to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub outcome: VerificationOutcome,
    /// Full stdout + stderr from the build process.
    pub build_logs: String,
    /// The SHA-256 hex digest we computed from the compiled WASM (if
    /// compilation succeeded).
    pub computed_hash: Option<String>,
}

// ─── Public Entry Point ──────────────────────────────────────────────────────

/// Verify that the supplied source compiles to the expected on-chain bytecode.
///
/// Wraps [`do_verify`] in a 5-minute timeout. Returns
/// [`RegistryError::Internal`] if the timeout is reached.
pub async fn verify_contract(request: VerificationRequest) -> Result<VerificationResult, RegistryError> {
    timeout(VERIFICATION_TIMEOUT, do_verify(request))
        .await
        .map_err(|_| {
            RegistryError::Internal(
                "Verification timed out after 5 minutes".to_string(),
            )
        })?
}

// ─── Core Pipeline ───────────────────────────────────────────────────────────

async fn do_verify(request: VerificationRequest) -> Result<VerificationResult, RegistryError> {
    // Normalise the expected hash before any comparison.
    let expected_hash = normalize_hash(&request.expected_wasm_hash).ok_or_else(|| {
        RegistryError::InvalidInput(
            "expected_wasm_hash must be a 64-char hex string".to_string(),
        )
    })?;

    // Keep the temp dir alive for the full duration of the function.
    let tmp_dir = TempDir::new()
        .map_err(|e| RegistryError::Internal(format!("Failed to create temp dir: {e}")))?;

    let mut logs = String::new();

    // 1. Obtain source ────────────────────────────────────────────────────────
    let wasm_bytes = match request.source {
        SourceInput::GitUrl { url, branch, commit } => {
            let source_dir = clone_repo(
                &url,
                branch.as_deref(),
                commit.as_deref(),
                tmp_dir.path(),
                &mut logs,
            )
            .await?;
            compile_contract(
                &source_dir,
                request.compiler_version.as_deref(),
                request.build_params.as_ref(),
                &mut logs,
            )
            .await?
        }

        SourceInput::SourceCode { code } => {
            // Special prefix: pre-compiled WASM encoded in base64 (useful for tests).
            if let Some(encoded) = code.trim().strip_prefix("wasm_base64:") {
                logs.push_str("Decoding pre-compiled WASM from wasm_base64 payload.\n");
                BASE64.decode(encoded.trim()).map_err(|e| {
                    RegistryError::InvalidInput(format!("Invalid wasm_base64 payload: {e}"))
                })?
            } else {
                let source_dir =
                    write_source_to_dir(&code, tmp_dir.path(), &mut logs).await?;
                compile_contract(
                    &source_dir,
                    request.compiler_version.as_deref(),
                    request.build_params.as_ref(),
                    &mut logs,
                )
                .await?
            }
        }
    };

    // 2. Hash the compiled WASM ───────────────────────────────────────────────
    let computed_hash = compute_wasm_hash(&wasm_bytes);
    logs.push_str(&format!(
        "\nComputed WASM hash : {computed_hash}\nExpected WASM hash : {expected_hash}\n"
    ));

    // 3. Compare ──────────────────────────────────────────────────────────────
    let outcome = if computed_hash == expected_hash {
        logs.push_str("Result             : VERIFIED ✓\n");
        VerificationOutcome::Verified
    } else {
        logs.push_str("Result             : FAILED — hash mismatch\n");
        VerificationOutcome::Failed {
            reason: format!(
                "Hash mismatch: compiled WASM produced {computed_hash} but on-chain hash is {expected_hash}"
            ),
        }
    };

    Ok(VerificationResult {
        outcome,
        build_logs: logs,
        computed_hash: Some(computed_hash),
    })
}

// ─── Source Acquisition ──────────────────────────────────────────────────────

/// Clone a public Git repository into `base_dir/repo`.
async fn clone_repo(
    url: &str,
    branch: Option<&str>,
    commit: Option<&str>,
    base_dir: &Path,
    logs: &mut String,
) -> Result<PathBuf, RegistryError> {
    logs.push_str(&format!("Cloning repository: {url}\n"));

    let repo_dir = base_dir.join("repo");

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth=1");
    if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(url).arg(&repo_dir);

    let output = cmd
        .output()
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to run git clone: {e}")))?;

    logs.push_str(&String::from_utf8_lossy(&output.stdout));
    logs.push_str(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        return Err(RegistryError::Internal(format!(
            "git clone failed (exit {}): {}",
            output.status,
            truncate_for_error(&String::from_utf8_lossy(&output.stderr))
        )));
    }

    if let Some(sha) = commit {
        logs.push_str(&format!("Checking out commit: {sha}\n"));
        let checkout = Command::new("git")
            .args(["checkout", sha])
            .current_dir(&repo_dir)
            .output()
            .await
            .map_err(|e| RegistryError::Internal(format!("Failed to run git checkout: {e}")))?;

        logs.push_str(&String::from_utf8_lossy(&checkout.stderr));

        if !checkout.status.success() {
            return Err(RegistryError::Internal(format!(
                "git checkout {sha} failed: {}",
                truncate_for_error(&String::from_utf8_lossy(&checkout.stderr))
            )));
        }
    }

    logs.push_str("Repository ready.\n");
    Ok(repo_dir)
}

/// Write a single-file Rust contract into a minimal Cargo project at
/// `base_dir/contract`.
async fn write_source_to_dir(
    source_code: &str,
    base_dir: &Path,
    logs: &mut String,
) -> Result<PathBuf, RegistryError> {
    let contract_dir = base_dir.join("contract");
    let src_dir = contract_dir.join("src");

    tokio::fs::create_dir_all(&src_dir)
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to create source dir: {e}")))?;

    tokio::fs::write(src_dir.join("lib.rs"), source_code)
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to write lib.rs: {e}")))?;

    let sdk_version = DEFAULT_SOROBAN_SDK_VERSION;
    let cargo_toml = format!(
        "[package]\nname = \"soroban-contract\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [lib]\ncrate-type = [\"cdylib\"]\n\n\
         [dependencies]\nsoroban-sdk = \"{sdk_version}\"\n\n\
         [profile.release]\nopt-level = \"z\"\noverflow-checks = true\ndebug = 0\n\
         strip = \"symbols\"\ndebug-assertions = false\npanic = \"abort\"\ncodegen-units = 1\nlto = true\n"
    );

    tokio::fs::write(contract_dir.join("Cargo.toml"), cargo_toml)
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to write Cargo.toml: {e}")))?;

    logs.push_str("Source code written to temporary directory.\n");
    Ok(contract_dir)
}

// ─── Compilation ─────────────────────────────────────────────────────────────

/// Compile the Cargo project at `source_dir` for the
/// `wasm32-unknown-unknown` target and return the raw WASM bytes.
async fn compile_contract(
    source_dir: &Path,
    _compiler_version: Option<&str>,
    build_params: Option<&Value>,
    logs: &mut String,
) -> Result<Vec<u8>, RegistryError> {
    if !source_dir.join("Cargo.toml").exists() {
        return Err(RegistryError::Internal(
            "No Cargo.toml found. Only Rust/Soroban contracts are currently supported.".to_string(),
        ));
    }

    logs.push_str("Starting compilation (cargo build --target wasm32-unknown-unknown --release)...\n");

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .current_dir(source_dir)
        .env("CARGO_TERM_COLOR", "never");

    if let Some(params) = build_params {
        apply_build_params(&mut cmd, params);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to spawn cargo: {e}")))?;

    logs.push_str(&String::from_utf8_lossy(&output.stdout));
    logs.push_str(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        return Err(RegistryError::Internal(format!(
            "Compilation failed (exit {}). See build logs for details.",
            output.status
        )));
    }

    let wasm_dir = source_dir.join("target/wasm32-unknown-unknown/release");
    let wasm_path = find_wasm_file(&wasm_dir).await?;

    logs.push_str(&format!("Compiled artefact  : {}\n", wasm_path.display()));

    let wasm_bytes = tokio::fs::read(&wasm_path)
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to read WASM file: {e}")))?;

    logs.push_str(&format!("WASM size          : {} bytes\n", wasm_bytes.len()));

    Ok(wasm_bytes)
}

/// Find the first `.wasm` file inside `dir`.
async fn find_wasm_file(dir: &Path) -> Result<PathBuf, RegistryError> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to read WASM output dir: {e}")))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| RegistryError::Internal(format!("Failed to iterate WASM dir: {e}")))?
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "wasm") {
            return Ok(path);
        }
    }

    Err(RegistryError::Internal(
        "No .wasm file found after compilation. Ensure crate-type includes \"cdylib\".".to_string(),
    ))
}

/// Apply optional build parameters (profile, features) to a cargo command.
fn apply_build_params(cmd: &mut Command, build_params: &Value) {
    if let Some(profile) = build_params.get("profile").and_then(Value::as_str) {
        cmd.arg("--profile").arg(profile);
    }
    if let Some(features) = build_params.get("features").and_then(Value::as_array) {
        let joined = features
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(",");
        if !joined.is_empty() {
            cmd.arg("--features").arg(joined);
        }
    }
}

// ─── Hashing & Utility ───────────────────────────────────────────────────────

/// Compute the SHA-256 hex digest of raw WASM bytes.
///
/// This mirrors how Stellar stores wasm_hash on-chain.
pub fn compute_wasm_hash(wasm_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(wasm_bytes);
    hex::encode(hasher.finalize())
}

/// Alias kept for callers using the upstream naming convention.
pub fn hash_wasm(wasm_bytes: &[u8]) -> String {
    compute_wasm_hash(wasm_bytes)
}

/// Normalise a WASM hash value: strip a leading `0x`, lowercase, and validate
/// that the result is exactly 64 hex characters.  Returns `None` on failure.
pub fn normalize_hash(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let stripped = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if stripped.len() != 64 || !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(stripped.to_ascii_lowercase())
}

fn truncate_for_error(value: &str) -> String {
    const MAX_ERROR_LEN: usize = 1_000;
    if value.len() <= MAX_ERROR_LEN {
        return value.to_string();
    }
    let mut out = value[..MAX_ERROR_LEN].to_string();
    out.push_str("...[truncated]");
    out
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Hash utilities ────────────────────────────────────────────────────────

    #[test]
    fn test_compute_wasm_hash_deterministic() {
        let bytes = b"mock wasm bytes";
        assert_eq!(compute_wasm_hash(bytes), compute_wasm_hash(bytes));
    }

    #[test]
    fn test_compute_wasm_hash_length() {
        assert_eq!(compute_wasm_hash(b"test").len(), 64);
    }

    #[test]
    fn test_compute_wasm_hash_differs() {
        assert_ne!(compute_wasm_hash(b"aaa"), compute_wasm_hash(b"bbb"));
    }

    #[test]
    fn test_hash_wasm_alias() {
        let b = b"soroban";
        assert_eq!(hash_wasm(b), compute_wasm_hash(b));
    }

    #[test]
    fn test_normalize_hash_valid() {
        let h = "a".repeat(64);
        assert_eq!(normalize_hash(&h), Some(h.clone()));
    }

    #[test]
    fn test_normalize_hash_strips_0x() {
        let inner = "b".repeat(64);
        let with_prefix = format!("0x{inner}");
        assert_eq!(normalize_hash(&with_prefix), Some(inner));
    }

    #[test]
    fn test_normalize_hash_lowercases() {
        let upper = "A".repeat(64);
        let lower = "a".repeat(64);
        assert_eq!(normalize_hash(&upper), Some(lower));
    }

    #[test]
    fn test_normalize_hash_rejects_wrong_length() {
        assert_eq!(normalize_hash("abc"), None);
    }

    #[test]
    fn test_normalize_hash_rejects_non_hex() {
        let bad = "g".repeat(64);
        assert_eq!(normalize_hash(&bad), None);
    }

    // ── wasm_base64 shortcut ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_verify_wasm_base64_match() {
        let wasm = b"known-good-wasm";
        let expected_hash = compute_wasm_hash(wasm);
        let source = format!("wasm_base64:{}", BASE64.encode(wasm));

        let req = VerificationRequest {
            source: SourceInput::SourceCode { code: source },
            compiler_version: None,
            build_params: None,
            expected_wasm_hash: expected_hash.clone(),
        };

        let result = verify_contract(req).await.expect("should succeed");
        assert_eq!(result.outcome, VerificationOutcome::Verified);
        assert_eq!(result.computed_hash.unwrap(), expected_hash);
    }

    #[tokio::test]
    async fn test_verify_wasm_base64_mismatch() {
        let source = format!("wasm_base64:{}", BASE64.encode(b"known-bad-wasm"));
        let wrong_hash = compute_wasm_hash(b"different-wasm");

        let req = VerificationRequest {
            source: SourceInput::SourceCode { code: source },
            compiler_version: None,
            build_params: None,
            expected_wasm_hash: wrong_hash,
        };

        let result = verify_contract(req).await.expect("should complete");
        assert!(
            matches!(result.outcome, VerificationOutcome::Failed { .. }),
            "expected Failed outcome on mismatch"
        );
    }

    // ── Git URL: invalid URL returns error ────────────────────────────────────

    #[tokio::test]
    async fn test_verify_invalid_git_url_returns_error() {
        let req = VerificationRequest {
            source: SourceInput::GitUrl {
                url: "https://invalid.example.invalid/repo.git".to_string(),
                branch: None,
                commit: None,
            },
            compiler_version: None,
            build_params: None,
            // Use a properly formed hash so normalize_hash passes.
            expected_wasm_hash: "a".repeat(64),
        };

        let result = verify_contract(req).await;
        assert!(result.is_err(), "Expected error for invalid Git URL");
    }

    // ── Upload: empty source produces Failed outcome ──────────────────────────

    #[tokio::test]
    async fn test_verify_source_upload_empty_source_fails() {
        let req = VerificationRequest {
            source: SourceInput::SourceCode {
                code: String::new(),
            },
            compiler_version: None,
            build_params: None,
            expected_wasm_hash: "a".repeat(64),
        };

        match verify_contract(req).await {
            Ok(result) => {
                assert!(
                    matches!(result.outcome, VerificationOutcome::Failed { .. }),
                    "Expected Failed outcome for empty source"
                );
            }
            Err(_) => {
                // Also acceptable if cargo is not on PATH.
            }
        }
    }
}
