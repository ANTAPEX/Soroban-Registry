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

#[allow(dead_code)]
pub mod bindings;
#[allow(dead_code)]
pub mod openapi;
#[allow(dead_code)]
pub mod parser;
#[allow(dead_code)]
pub mod types;
#[allow(dead_code)]
pub mod validator;

#[allow(unused_imports)]
pub use bindings::*;
#[allow(unused_imports)]
pub use openapi::*;
#[allow(unused_imports)]
pub use parser::*;
#[allow(unused_imports)]
pub use types::*;
#[allow(unused_imports)]
pub use validator::*;
