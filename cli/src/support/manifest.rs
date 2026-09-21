#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub schema_version: String,
    pub contract_id: String,
    pub name: String,
    pub network: String,
    pub exported_at: DateTime<Utc>,
    pub sha256: String,
    pub contents: Vec<ManifestEntry>,
    pub audit_trail: Vec<AuditEntry>,
    pub signature: Option<PackageSignatureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSignatureInfo {
    pub signature: String,
    pub signing_address: String,
    pub public_key: String,
    pub algorithm: String,
    pub signed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub key_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub size: u64,
    pub modified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
}

impl ExportManifest {
    pub fn new(contract_id: String, name: String, network: String) -> Self {
        Self {
            schema_version: "1.0".into(),
            contract_id,
            name,
            network,
            exported_at: Utc::now(),
            sha256: String::new(),
            contents: Vec::new(),
            audit_trail: vec![AuditEntry {
                action: "export_created".into(),
                timestamp: Utc::now(),
                actor: "soroban-registry-cli".into(),
            }],
            signature: None,
        }
    }

    pub fn with_signature(mut self, sig_info: PackageSignatureInfo) -> Self {
        self.signature = Some(sig_info);
        self.audit_trail.push(AuditEntry {
            action: "package_signed".into(),
            timestamp: Utc::now(),
            actor: "soroban-registry-cli".into(),
        });
        self
    }
}
