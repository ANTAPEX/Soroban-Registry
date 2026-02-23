//! Contract Type Safety Validator
//!
//! This module provides type safety validation for contract function calls
//! before submission, preventing runtime errors.
//!
//! Features:
//! - Parse contract ABI and expected types
//! - Validate parameters against contract spec
//! - Check function existence and visibility
//! - Return type validation
//! - Generate TypeScript/Rust bindings

pub mod types;
pub mod parser;
pub mod validator;
pub mod bindings;
pub mod openapi;

pub use types::*;
pub use parser::*;
pub use validator::*;
pub use bindings::*;
pub use openapi::*;
