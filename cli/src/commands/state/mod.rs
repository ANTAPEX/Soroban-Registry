//! `soroban-registry state ...` — a contract's key/value state.
//!
//! The registry is asked first and the local store is the fallback, so these
//! work offline: `store` owns that file and `remote` the HTTP side.

pub mod dump;
pub mod get;
pub mod history;
pub mod remote;
pub mod set;
pub mod snapshot;
pub mod store;
