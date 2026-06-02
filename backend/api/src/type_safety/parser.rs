//! ABI Parser for Soroban contracts
//!
//! Parses contract ABI from various sources (WASM, JSON spec, etc.)
//! into our internal ContractABI representation.

use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw contract spec from soroban CLI bindings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawContractSpec {
    #[serde(rename = "type")]
    pub spec_type: String,
    pub name: String,
    #[serde(default)]
    pub inputs: Vec<RawInputSpec>,
    #[serde(default)]
    pub outputs: Vec<RawOutputSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default)]
    pub fields: Vec<RawFieldSpec>,
    #[serde(default)]
    pub cases: Vec<RawEnumCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInputSpec {
    pub name: String,
    pub value: RawTypeValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawOutputSpec {
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTypeValue {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<Box<RawTypeValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<Box<RawTypeValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<Box<RawTypeValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFieldSpec {
    pub name: String,
    pub value: RawTypeValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEnumCase {
    pub name: String,
    pub value: Option<u32>,
    #[serde(default)]
    pub fields: Vec<RawFieldSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// ABI Parser errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    pub context: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            context: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ctx) = &self.context {
            write!(f, "{}: {}", ctx, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse raw contract specs into a ContractABI
pub fn parse_contract_abi(
    specs: &[RawContractSpec],
    contract_name: &str,
) -> Result<ContractABI, ParseError> {
    let mut abi = ContractABI::new(contract_name.to_string());

    // First pass: collect all type definitions (structs, enums)
    for spec in specs {
        match spec.spec_type.as_str() {
            "struct" => {
                let struct_type = parse_struct_type(spec)?;
                abi.types.insert(spec.name.clone(), struct_type);
            }
            "union" | "enum" => {
                let enum_type = parse_enum_type(spec)?;
                abi.types.insert(spec.name.clone(), enum_type);
            }
            "error_enum" => {
                // Error enums are also types
                let enum_type = parse_error_enum(spec)?;
                abi.types.insert(spec.name.clone(), enum_type.clone());

                // Also add to errors list
                if let SorobanType::Enum { variants, .. } = enum_type {
                    for variant in variants {
                        abi.errors.push(ContractError {
                            name: format!("{}::{}", spec.name, variant.name),
                            code: variant.value.unwrap_or(0),
                            doc: variant.doc,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // Second pass: parse functions with resolved types
    for spec in specs {
        if spec.spec_type == "function" {
            let func = parse_function(spec, &abi.types)?;
            abi.functions.push(func);
        }
    }

    Ok(abi)
}

/// Parse a struct type specification
fn parse_struct_type(spec: &RawContractSpec) -> Result<SorobanType, ParseError> {
    let fields: Vec<StructField> = spec
        .fields
        .iter()
        .map(|f| StructField {
            name: f.name.clone(),
            field_type: parse_type_value(&f.value),
            doc: f.doc.clone(),
        })
        .collect();

    Ok(SorobanType::Struct {
        name: spec.name.clone(),
        fields,
    })
}

/// Parse an enum type specification
fn parse_enum_type(spec: &RawContractSpec) -> Result<SorobanType, ParseError> {
    let variants: Vec<EnumVariant> = spec
        .cases
        .iter()
        .map(|c| {
            let fields = if c.fields.is_empty() {
                None
            } else {
                Some(
                    c.fields
                        .iter()
                        .map(|f| StructField {
                            name: f.name.clone(),
                            field_type: parse_type_value(&f.value),
                            doc: f.doc.clone(),
                        })
                        .collect(),
                )
            };
            EnumVariant {
                name: c.name.clone(),
                value: c.value,
                fields,
                doc: c.doc.clone(),
            }
        })
        .collect();

    Ok(SorobanType::Enum {
        name: spec.name.clone(),
        variants,
    })
}

/// Parse an error enum specification
fn parse_error_enum(spec: &RawContractSpec) -> Result<SorobanType, ParseError> {
    parse_enum_type(spec)
}

/// Parse a function specification
fn parse_function(
    spec: &RawContractSpec,
    _types: &HashMap<String, SorobanType>,
) -> Result<ContractFunction, ParseError> {
    let params: Vec<FunctionParam> = spec
        .inputs
        .iter()
        .map(|input| FunctionParam {
            name: input.name.clone(),
            param_type: parse_type_value(&input.value),
            doc: input.doc.clone(),
        })
        .collect();

    let return_type = spec
        .outputs
        .first()
        .map(|o| SorobanType::from_type_string(&o.type_name))
        .unwrap_or(SorobanType::Void);

    // Determine if function is mutable based on naming conventions
    // or if it has state-changing parameters
    let is_mutable = !spec.name.starts_with("get_")
        && !spec.name.starts_with("view_")
        && !spec.name.starts_with("query_")
        && !spec.name.starts_with("is_")
        && !spec.name.starts_with("has_");

    Ok(ContractFunction {
        name: spec.name.clone(),
        visibility: FunctionVisibility::Public, // Soroban exported functions are public
        params,
        return_type,
        doc: spec.doc.clone(),
        is_mutable,
    })
}

/// Parse a raw type value into SorobanType
fn parse_type_value(value: &RawTypeValue) -> SorobanType {
    match value.type_name.to_lowercase().as_str() {
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
        "option" => {
            let inner = value
                .element
                .as_ref()
                .map(|e| parse_type_value(e))
                .unwrap_or(SorobanType::Void);
            SorobanType::Option {
                value_type: Box::new(inner),
            }
        }
        "vec" => {
            let elem = value
                .element
                .as_ref()
                .map(|e| parse_type_value(e))
                .unwrap_or(SorobanType::Void);
            SorobanType::Vec {
                element_type: Box::new(elem),
            }
        }
        "map" => {
            let key = value
                .key
                .as_ref()
                .map(|k| parse_type_value(k))
                .unwrap_or(SorobanType::Void);
            let val = value
                .val
                .as_ref()
                .map(|v| parse_type_value(v))
                .unwrap_or(SorobanType::Void);
            SorobanType::Map {
                key_type: Box::new(key),
                value_type: Box::new(val),
            }
        }
        "bytesn" => {
            let n = value.n.unwrap_or(32);
            SorobanType::BytesN { n }
        }
        _ => SorobanType::Custom {
            name: value.type_name.clone(),
        },
    }
}

/// Parse JSON spec string into ContractABI
pub fn parse_json_spec(json: &str, contract_name: &str) -> Result<ContractABI, ParseError> {
    let specs: Vec<RawContractSpec> = serde_json::from_str(json)
        .map_err(|e| ParseError::new(format!("Failed to parse JSON: {}", e)))?;

    parse_contract_abi(&specs, contract_name)
}

/// Parse value string into ParsedValue based on expected type
#[allow(dead_code)]
pub fn parse_value_string(
    value: &str,
    expected_type: &SorobanType,
) -> Result<ParsedValue, ParseError> {
    let trimmed = value.trim();

    match expected_type {
        SorobanType::Bool => match trimmed.to_lowercase().as_str() {
            "true" | "1" => Ok(ParsedValue::Bool(true)),
            "false" | "0" => Ok(ParsedValue::Bool(false)),
            _ => Err(ParseError::new(format!(
                "Invalid boolean value: '{}'. Expected 'true' or 'false'",
                trimmed
            ))),
        },
        SorobanType::I32 | SorobanType::I64 | SorobanType::I128 | SorobanType::I256 => trimmed
            .parse::<i128>()
            .map(ParsedValue::Integer)
            .map_err(|_| ParseError::new(format!("Invalid integer value: '{}'", trimmed))),
        SorobanType::U32 | SorobanType::U64 | SorobanType::U128 | SorobanType::U256 => trimmed
            .parse::<u128>()
            .map(ParsedValue::UnsignedInteger)
            .map_err(|_| ParseError::new(format!("Invalid unsigned integer value: '{}'", trimmed))),
        SorobanType::String => Ok(ParsedValue::String(trimmed.to_string())),
        SorobanType::Symbol => {
            // Symbols have restrictions: alphanumeric and underscore, max 32 chars
            if trimmed.len() > 32 {
                return Err(ParseError::new(
                    "Symbol exceeds maximum length of 32 characters",
                ));
            }
            if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(ParseError::new(
                    "Symbol must contain only alphanumeric characters and underscores",
                ));
            }
            Ok(ParsedValue::Symbol(trimmed.to_string()))
        }
        SorobanType::Address => {
            // Validate Stellar address format (starts with G or C, 56 chars)
            if trimmed.len() != 56 {
                return Err(ParseError::new("Address must be exactly 56 characters"));
            }
            if !trimmed.starts_with('G') && !trimmed.starts_with('C') {
                return Err(ParseError::new(
                    "Address must start with 'G' (account) or 'C' (contract)",
                ));
            }
            Ok(ParsedValue::Address(trimmed.to_string()))
        }
        SorobanType::Bytes => {
            // Parse hex string to bytes
            let bytes = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                hex::decode(&trimmed[2..])
            } else {
                hex::decode(trimmed)
            };
            bytes
                .map(ParsedValue::Bytes)
                .map_err(|_| ParseError::new(format!("Invalid hex bytes: '{}'", trimmed)))
        }
        SorobanType::BytesN { n } => {
            let bytes = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                hex::decode(&trimmed[2..])
            } else {
                hex::decode(trimmed)
            };
            let bytes =
                bytes.map_err(|_| ParseError::new(format!("Invalid hex bytes: '{}'", trimmed)))?;
            if bytes.len() != *n as usize {
                return Err(ParseError::new(format!(
                    "Expected {} bytes, got {}",
                    n,
                    bytes.len()
                )));
            }
            Ok(ParsedValue::Bytes(bytes))
        }
        SorobanType::Void => Ok(ParsedValue::Null),
        SorobanType::Vec { element_type } => {
            // Parse JSON array
            let arr: Vec<serde_json::Value> = serde_json::from_str(trimmed)
                .map_err(|_| ParseError::new("Invalid array format"))?;
            let mut parsed = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let value_str = item.to_string();
                let parsed_item = parse_value_string(&value_str, element_type)
                    .map_err(|e| e.with_context(format!("Array element {}", i)))?;
                parsed.push(parsed_item);
            }
            Ok(ParsedValue::Array(parsed))
        }
        SorobanType::Option { value_type } => {
            if trimmed.is_empty() || trimmed == "null" || trimmed == "None" {
                Ok(ParsedValue::Null)
            } else {
                parse_value_string(trimmed, value_type)
            }
        }
        _ => {
            // For complex types, try JSON parsing
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                parse_json_value(&json_val, expected_type)
            } else {
                Ok(ParsedValue::String(trimmed.to_string()))
            }
        }
    }
}

/// Parse a JSON value into ParsedValue
fn parse_json_value(
    value: &serde_json::Value,
    _expected_type: &SorobanType,
) -> Result<ParsedValue, ParseError> {
    match value {
        serde_json::Value::Null => Ok(ParsedValue::Null),
        serde_json::Value::Bool(b) => Ok(ParsedValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ParsedValue::Integer(i as i128))
            } else if let Some(u) = n.as_u64() {
                Ok(ParsedValue::UnsignedInteger(u as u128))
            } else {
                Err(ParseError::new("Number out of range"))
            }
        }
        serde_json::Value::String(s) => Ok(ParsedValue::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let mut parsed = Vec::new();
            for item in arr {
                parsed.push(parse_json_value(item, &SorobanType::Void)?);
            }
            Ok(ParsedValue::Array(parsed))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), parse_json_value(v, &SorobanType::Void)?);
            }
            Ok(ParsedValue::Struct(map))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool() {
        let result = parse_value_string("true", &SorobanType::Bool).unwrap();
        assert!(matches!(result, ParsedValue::Bool(true)));

        let result = parse_value_string("false", &SorobanType::Bool).unwrap();
        assert!(matches!(result, ParsedValue::Bool(false)));
    }

    #[test]
    fn test_parse_integers() {
        let result = parse_value_string("42", &SorobanType::I32).unwrap();
        assert!(matches!(result, ParsedValue::Integer(42)));

        let result = parse_value_string("-100", &SorobanType::I64).unwrap();
        assert!(matches!(result, ParsedValue::Integer(-100)));

        let result = parse_value_string("1000000", &SorobanType::U64).unwrap();
        assert!(matches!(result, ParsedValue::UnsignedInteger(1000000)));
    }

    #[test]
    fn test_parse_address() {
        let valid = "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        let result = parse_value_string(valid, &SorobanType::Address).unwrap();
        assert!(matches!(result, ParsedValue::Address(_)));

        // Invalid: wrong length
        let invalid = "GDLZFC3S";
        assert!(parse_value_string(invalid, &SorobanType::Address).is_err());
    }

    #[test]
    fn test_parse_symbol() {
        let result = parse_value_string("transfer", &SorobanType::Symbol).unwrap();
        assert!(matches!(result, ParsedValue::Symbol(_)));

        // Invalid: too long
        let long = "a".repeat(33);
        assert!(parse_value_string(&long, &SorobanType::Symbol).is_err());
    }

    #[test]
    fn test_parse_json_spec() {
        let json = r#"[
            {
                "type": "function",
                "name": "transfer",
                "inputs": [
                    {"name": "to", "value": {"type": "Address"}},
                    {"name": "amount", "value": {"type": "i128"}}
                ],
                "outputs": [{"type": "bool"}]
            }
        ]"#;

        let abi = parse_json_spec(json, "TestToken").unwrap();
        assert_eq!(abi.name, "TestToken");
        assert!(abi.has_function("transfer"));

        let func = abi.find_function("transfer").unwrap();
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "to");
        assert!(matches!(func.params[0].param_type, SorobanType::Address));
    }
}
