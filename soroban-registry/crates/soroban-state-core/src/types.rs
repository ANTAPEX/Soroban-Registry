/// Shared types for state inspection
use serde::{Deserialize, Serialize};

/// Decoded value from XDR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum DecodedValue {
    Bool(bool),
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Int128(i128),
    Uint128(u128),
    Bytes(String), // hex-encoded
    String(String),
    Symbol(String),
    Address(String), // Stellar strkey format
    Map(Vec<(Box<DecodedValue>, Box<DecodedValue>)>),
    Vec(Vec<DecodedValue>),
    Void,
    Error(String),
    Unknown(String), // fallback with raw XDR hex
}

impl std::fmt::Display for DecodedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodedValue::Bool(b) => write!(f, "{}", b),
            DecodedValue::Int32(n) => write!(f, "{}", n),
            DecodedValue::Uint32(n) => write!(f, "{}", n),
            DecodedValue::Int64(n) => write!(f, "{}", n),
            DecodedValue::Uint64(n) => write!(f, "{}", n),
            DecodedValue::Int128(n) => write!(f, "{}", n),
            DecodedValue::Uint128(n) => write!(f, "{}", n),
            DecodedValue::Bytes(s) => write!(f, "0x{}", s),
            DecodedValue::String(s) => write!(f, "\"{}\"", s),
            DecodedValue::Symbol(s) => write!(f, ":{}", s),
            DecodedValue::Address(a) => write!(f, "{}", a),
            DecodedValue::Map(_) => write!(f, "{{...}}"),
            DecodedValue::Vec(v) => write!(f, "[...({} items)]", v.len()),
            DecodedValue::Void => write!(f, "void"),
            DecodedValue::Error(e) => write!(f, "error({})", e),
            DecodedValue::Unknown(u) => write!(f, "unknown({})", u),
        }
    }
}

/// Type of ledger entry
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Persistent,
    Temporary,
    Instance,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::Persistent => write!(f, "Persistent"),
            EntryType::Temporary => write!(f, "Temporary"),
            EntryType::Instance => write!(f, "Instance"),
        }
    }
}

/// Single state entry (key-value pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    pub key: DecodedValue,
    pub key_raw: String,
    pub value: DecodedValue,
    pub value_raw: String,
    pub entry_type: EntryType,
    pub ttl: Option<u32>,
}

/// Complete contract state snapshot
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractState {
    pub contract_id: String,
    pub ledger: u32,
    pub timestamp: String,
    pub entries: Vec<StateEntry>,
}

/// State snapshot for history
#[derive(Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub ledger: u32,
    pub timestamp: String,
    pub entries: Vec<StateEntry>,
}

/// Modified entry for diffs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedEntry {
    pub key: DecodedValue,
    pub before: DecodedValue,
    pub after: DecodedValue,
}

/// Complete state diff
#[derive(Debug, Serialize, Deserialize)]
pub struct StateDiff {
    pub contract_id: String,
    pub from_ledger: u32,
    pub to_ledger: u32,
    pub added: Vec<StateEntry>,
    pub removed: Vec<StateEntry>,
    pub modified: Vec<ModifiedEntry>,
    pub unchanged: usize,
}

/// Result of a dry run simulation
#[derive(Debug, Serialize, Deserialize)]
pub struct DryRunResult {
    pub success: bool,
    pub return_value: Option<DecodedValue>,
    pub state_changes: Vec<ModifiedEntry>,
    pub events: Vec<String>,
    pub cpu_instructions: u64,
    pub memory_bytes: u64,
    pub error: Option<String>,
}

/// Contract event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub ledger: u32,
    pub tx_index: u32,
    pub event_index: u32,
    pub event_type: String,
    pub data: Vec<String>,
}

/// Response from Stellar RPC for ledger entries
#[derive(Debug, Deserialize)]
pub struct LedgerEntriesResponse {
    pub ledger_entries: Option<Vec<LedgerEntry>>,
    #[serde(rename = "latestLedger")]
    pub latest_ledger: u32,
    #[serde(rename = "latestLedgerCloseTime")]
    pub latest_ledger_close_time: u64,
}

#[derive(Debug, Deserialize)]
pub struct LedgerEntry {
    pub key: String,
    pub xdr: String,
    #[serde(rename = "lastModifiedLedgerSeq")]
    pub last_modified_ledger_seq: u32,
}
