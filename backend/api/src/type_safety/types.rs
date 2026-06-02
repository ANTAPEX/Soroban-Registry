//! Type definitions for Soroban contract type safety validation
//!
//! Defines the type system used for validating contract function calls.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Soroban native types supported in contracts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SorobanType {
    /// Boolean type
    Bool,
    /// Signed 32-bit integer
    I32,
    /// Signed 64-bit integer
    I64,
    /// Signed 128-bit integer
    I128,
    /// Signed 256-bit integer
    I256,
    /// Unsigned 32-bit integer
    U32,
    /// Unsigned 64-bit integer
    U64,
    /// Unsigned 128-bit integer
    U128,
    /// Unsigned 256-bit integer
    U256,
    /// Symbol (short string identifier)
    Symbol,
    /// String type
    String,
    /// Bytes (raw byte array)
    Bytes,
    /// Fixed-length bytes
    BytesN { n: u32 },
    /// Address (account or contract)
    Address,
    /// Void (no value)
    Void,
    /// Timepoint (timestamp)
    Timepoint,
    /// Duration
    Duration,
    /// Option type (nullable)
    Option { value_type: Box<SorobanType> },
    /// Result type
    Result {
        ok_type: Box<SorobanType>,
        err_type: Box<SorobanType>,
    },
    /// Vector (dynamic array)
    Vec { element_type: Box<SorobanType> },
    /// Map type
    Map {
        key_type: Box<SorobanType>,
        value_type: Box<SorobanType>,
    },
    /// Tuple type
    Tuple { elements: Vec<SorobanType> },
    /// Struct type (user-defined)
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    /// Enum type (user-defined)
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    /// Custom/unknown type reference
    Custom { name: String },
}

impl SorobanType {
    /// Parse a type string into SorobanType
    pub fn from_type_string(type_str: &str) -> Self {
        let trimmed = type_str.trim();

        match trimmed.to_lowercase().as_str() {
            "bool" => SorobanType::Bool,
            "i32" => SorobanType::I32,
            "i64" => SorobanType::I64,
            "i128" => SorobanType::I128,
            "i256" => SorobanType::I256,
            "u32" => SorobanType::U32,
            "u64" => SorobanType::U64,
            "u128" => SorobanType::U128,
            "u256" => SorobanType::U256,
            "symbol" => SorobanType::Symbol,
            "string" => SorobanType::String,
            "bytes" => SorobanType::Bytes,
            "address" => SorobanType::Address,
            "void" | "()" => SorobanType::Void,
            "timepoint" => SorobanType::Timepoint,
            "duration" => SorobanType::Duration,
            _ => {
                // Handle parameterized types
                if let Some(inner) = Self::extract_generic(trimmed, "Option") {
                    return SorobanType::Option {
                        value_type: Box::new(Self::from_type_string(&inner)),
                    };
                }
                if let Some(inner) = Self::extract_generic(trimmed, "Vec") {
                    return SorobanType::Vec {
                        element_type: Box::new(Self::from_type_string(&inner)),
                    };
                }
                if let Some(n) = Self::extract_bytes_n(trimmed) {
                    return SorobanType::BytesN { n };
                }
                // Default to custom type
                SorobanType::Custom {
                    name: trimmed.to_string(),
                }
            }
        }
    }

    /// Extract inner type from generic like Option<T> or Vec<T>
    fn extract_generic(type_str: &str, wrapper: &str) -> Option<String> {
        let prefix = format!("{}<", wrapper);
        if type_str.starts_with(&prefix) && type_str.ends_with('>') {
            let inner = &type_str[prefix.len()..type_str.len() - 1];
            Some(inner.to_string())
        } else {
            None
        }
    }

    /// Extract N from BytesN<N>
    fn extract_bytes_n(type_str: &str) -> Option<u32> {
        if type_str.starts_with("BytesN<") && type_str.ends_with('>') {
            let n_str = &type_str[7..type_str.len() - 1];
            n_str.parse().ok()
        } else {
            None
        }
    }

    /// Get the type name for display
    pub fn display_name(&self) -> String {
        match self {
            SorobanType::Bool => "bool".to_string(),
            SorobanType::I32 => "i32".to_string(),
            SorobanType::I64 => "i64".to_string(),
            SorobanType::I128 => "i128".to_string(),
            SorobanType::I256 => "i256".to_string(),
            SorobanType::U32 => "u32".to_string(),
            SorobanType::U64 => "u64".to_string(),
            SorobanType::U128 => "u128".to_string(),
            SorobanType::U256 => "u256".to_string(),
            SorobanType::Symbol => "Symbol".to_string(),
            SorobanType::String => "String".to_string(),
            SorobanType::Bytes => "Bytes".to_string(),
            SorobanType::BytesN { n } => format!("BytesN<{}>", n),
            SorobanType::Address => "Address".to_string(),
            SorobanType::Void => "void".to_string(),
            SorobanType::Timepoint => "Timepoint".to_string(),
            SorobanType::Duration => "Duration".to_string(),
            SorobanType::Option { value_type } => format!("Option<{}>", value_type.display_name()),
            SorobanType::Result { ok_type, err_type } => {
                format!(
                    "Result<{}, {}>",
                    ok_type.display_name(),
                    err_type.display_name()
                )
            }
            SorobanType::Vec { element_type } => format!("Vec<{}>", element_type.display_name()),
            SorobanType::Map {
                key_type,
                value_type,
            } => {
                format!(
                    "Map<{}, {}>",
                    key_type.display_name(),
                    value_type.display_name()
                )
            }
            SorobanType::Tuple { elements } => {
                let inner: Vec<String> = elements.iter().map(|e| e.display_name()).collect();
                format!("({})", inner.join(", "))
            }
            SorobanType::Struct { name, .. } => name.clone(),
            SorobanType::Enum { name, .. } => name.clone(),
            SorobanType::Custom { name } => name.clone(),
        }
    }

    /// Check if this type is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            SorobanType::I32
                | SorobanType::I64
                | SorobanType::I128
                | SorobanType::I256
                | SorobanType::U32
                | SorobanType::U64
                | SorobanType::U128
                | SorobanType::U256
        )
    }

    /// Check if this type is signed
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            SorobanType::I32 | SorobanType::I64 | SorobanType::I128 | SorobanType::I256
        )
    }
}

/// Struct field definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub field_type: SorobanType,
    pub doc: Option<String>,
}

/// Enum variant definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<u32>,
    pub fields: Option<Vec<StructField>>,
    pub doc: Option<String>,
}

/// Function visibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunctionVisibility {
    /// Public function callable externally
    #[default]
    Public,
    /// Internal function (not callable externally)
    Internal,
}

/// Function parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: SorobanType,
    pub doc: Option<String>,
}

/// Contract function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractFunction {
    pub name: String,
    pub visibility: FunctionVisibility,
    pub params: Vec<FunctionParam>,
    pub return_type: SorobanType,
    pub doc: Option<String>,
    pub is_mutable: bool,
}

/// Complete contract ABI specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractABI {
    pub name: String,
    pub version: Option<String>,
    pub functions: Vec<ContractFunction>,
    pub types: HashMap<String, SorobanType>,
    pub events: Vec<ContractEvent>,
    pub errors: Vec<ContractError>,
}

impl ContractABI {
    /// Create a new empty ABI
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            functions: Vec::new(),
            types: HashMap::new(),
            events: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Find a function by name
    pub fn find_function(&self, name: &str) -> Option<&ContractFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Get all public functions
    pub fn public_functions(&self) -> impl Iterator<Item = &ContractFunction> {
        self.functions
            .iter()
            .filter(|f| f.visibility == FunctionVisibility::Public)
    }

    /// Check if a function exists
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.iter().any(|f| f.name == name)
    }
}

/// Contract event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub name: String,
    pub topics: Vec<FunctionParam>,
    pub data: Vec<FunctionParam>,
    pub doc: Option<String>,
}

/// Contract error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractError {
    pub name: String,
    pub code: u32,
    pub doc: Option<String>,
}

/// Parsed parameter value for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParsedValue {
    Bool(bool),
    Integer(i128),
    UnsignedInteger(u128),
    String(String),
    Symbol(String),
    Bytes(Vec<u8>),
    Address(String),
    Array(Vec<ParsedValue>),
    Map(Vec<(ParsedValue, ParsedValue)>),
    Struct(HashMap<String, ParsedValue>),
    Null,
}

impl ParsedValue {
    /// Infer the Soroban type from this value
    pub fn infer_type(&self) -> SorobanType {
        match self {
            ParsedValue::Bool(_) => SorobanType::Bool,
            ParsedValue::Integer(n) => {
                if *n >= i32::MIN as i128 && *n <= i32::MAX as i128 {
                    SorobanType::I32
                } else if *n >= i64::MIN as i128 && *n <= i64::MAX as i128 {
                    SorobanType::I64
                } else {
                    SorobanType::I128
                }
            }
            ParsedValue::UnsignedInteger(n) => {
                if *n <= u32::MAX as u128 {
                    SorobanType::U32
                } else if *n <= u64::MAX as u128 {
                    SorobanType::U64
                } else {
                    SorobanType::U128
                }
            }
            ParsedValue::String(_) => SorobanType::String,
            ParsedValue::Symbol(_) => SorobanType::Symbol,
            ParsedValue::Bytes(_) => SorobanType::Bytes,
            ParsedValue::Address(_) => SorobanType::Address,
            ParsedValue::Array(items) => {
                let elem_type = items
                    .first()
                    .map(|v| v.infer_type())
                    .unwrap_or(SorobanType::Void);
                SorobanType::Vec {
                    element_type: Box::new(elem_type),
                }
            }
            ParsedValue::Map(entries) => {
                let (key_type, val_type) = entries
                    .first()
                    .map(|(k, v)| (k.infer_type(), v.infer_type()))
                    .unwrap_or((SorobanType::Void, SorobanType::Void));
                SorobanType::Map {
                    key_type: Box::new(key_type),
                    value_type: Box::new(val_type),
                }
            }
            ParsedValue::Struct(_) => SorobanType::Custom {
                name: "Struct".to_string(),
            },
            ParsedValue::Null => SorobanType::Void,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_types() {
        assert_eq!(SorobanType::from_type_string("bool"), SorobanType::Bool);
        assert_eq!(SorobanType::from_type_string("i32"), SorobanType::I32);
        assert_eq!(SorobanType::from_type_string("u64"), SorobanType::U64);
        assert_eq!(
            SorobanType::from_type_string("Address"),
            SorobanType::Address
        );
        assert_eq!(SorobanType::from_type_string("String"), SorobanType::String);
    }

    #[test]
    fn test_parse_option_type() {
        let opt = SorobanType::from_type_string("Option<u32>");
        assert!(matches!(opt, SorobanType::Option { .. }));
        if let SorobanType::Option { value_type } = opt {
            assert_eq!(*value_type, SorobanType::U32);
        }
    }

    #[test]
    fn test_parse_vec_type() {
        let vec = SorobanType::from_type_string("Vec<Address>");
        assert!(matches!(vec, SorobanType::Vec { .. }));
        if let SorobanType::Vec { element_type } = vec {
            assert_eq!(*element_type, SorobanType::Address);
        }
    }

    #[test]
    fn test_parse_bytes_n() {
        let bytes32 = SorobanType::from_type_string("BytesN<32>");
        assert!(matches!(bytes32, SorobanType::BytesN { n: 32 }));
    }

    #[test]
    fn test_display_name() {
        assert_eq!(SorobanType::Bool.display_name(), "bool");
        assert_eq!(SorobanType::BytesN { n: 32 }.display_name(), "BytesN<32>");
        assert_eq!(
            SorobanType::Option {
                value_type: Box::new(SorobanType::U64)
            }
            .display_name(),
            "Option<u64>"
        );
    }

    #[test]
    fn test_is_numeric() {
        assert!(SorobanType::I32.is_numeric());
        assert!(SorobanType::U128.is_numeric());
        assert!(!SorobanType::String.is_numeric());
        assert!(!SorobanType::Address.is_numeric());
    }

    #[test]
    fn test_contract_abi() {
        let mut abi = ContractABI::new("TestContract".to_string());
        abi.functions.push(ContractFunction {
            name: "transfer".to_string(),
            visibility: FunctionVisibility::Public,
            params: vec![
                FunctionParam {
                    name: "to".to_string(),
                    param_type: SorobanType::Address,
                    doc: None,
                },
                FunctionParam {
                    name: "amount".to_string(),
                    param_type: SorobanType::I128,
                    doc: None,
                },
            ],
            return_type: SorobanType::Bool,
            doc: Some("Transfer tokens".to_string()),
            is_mutable: true,
        });

        assert!(abi.has_function("transfer"));
        assert!(!abi.has_function("unknown"));

        let func = abi.find_function("transfer").unwrap();
        assert_eq!(func.params.len(), 2);
    }
}
