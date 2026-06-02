//! Contract Call Type Safety Validator
//!
//! Validates contract function calls for type safety before submission.

use super::parser::parse_value_string;
use super::types::*;
use serde::{Deserialize, Serialize};

/// Validation result for a contract call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub function_name: String,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub parsed_params: Option<Vec<ParsedParam>>,
    pub expected_return: Option<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success(
        function_name: String,
        params: Vec<ParsedParam>,
        return_type: &SorobanType,
    ) -> Self {
        Self {
            valid: true,
            function_name,
            errors: Vec::new(),
            warnings: Vec::new(),
            parsed_params: Some(params),
            expected_return: Some(return_type.display_name()),
        }
    }

    /// Create a failed validation result
    pub fn failure(function_name: String, errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            function_name,
            errors,
            warnings: Vec::new(),
            parsed_params: None,
            expected_return: None,
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: ErrorCode,
    pub message: String,
    pub field: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl ValidationError {
    pub fn function_not_found(name: &str) -> Self {
        Self {
            code: ErrorCode::FunctionNotFound,
            message: format!("Function '{}' not found in contract ABI", name),
            field: None,
            expected: None,
            actual: Some(name.to_string()),
        }
    }

    pub fn function_not_public(name: &str) -> Self {
        Self {
            code: ErrorCode::FunctionNotPublic,
            message: format!("Function '{}' is not publicly callable", name),
            field: None,
            expected: Some("public function".to_string()),
            actual: Some("internal function".to_string()),
        }
    }

    pub fn param_count_mismatch(expected: usize, actual: usize) -> Self {
        Self {
            code: ErrorCode::ParamCountMismatch,
            message: format!(
                "Expected {} parameters, but {} were provided",
                expected, actual
            ),
            field: None,
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
        }
    }

    pub fn type_mismatch(param_name: &str, expected: &str, actual: &str) -> Self {
        Self {
            code: ErrorCode::TypeMismatch,
            message: format!(
                "Parameter '{}': expected type '{}', got '{}'",
                param_name, expected, actual
            ),
            field: Some(param_name.to_string()),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
        }
    }

    pub fn parse_error(param_name: &str, message: &str) -> Self {
        Self {
            code: ErrorCode::ParseError,
            message: format!("Parameter '{}': {}", param_name, message),
            field: Some(param_name.to_string()),
            expected: None,
            actual: None,
        }
    }

    pub fn value_out_of_range(param_name: &str, expected_type: &str) -> Self {
        Self {
            code: ErrorCode::ValueOutOfRange,
            message: format!(
                "Parameter '{}': value out of range for type '{}'",
                param_name, expected_type
            ),
            field: Some(param_name.to_string()),
            expected: Some(expected_type.to_string()),
            actual: None,
        }
    }

    pub fn invalid_address(param_name: &str, address: &str) -> Self {
        Self {
            code: ErrorCode::InvalidAddress,
            message: format!(
                "Parameter '{}': '{}' is not a valid Stellar address",
                param_name, address
            ),
            field: Some(param_name.to_string()),
            expected: Some("Valid Stellar address (G... or C...)".to_string()),
            actual: Some(address.to_string()),
        }
    }
}

/// Error codes for validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    FunctionNotFound,
    FunctionNotPublic,
    ParamCountMismatch,
    TypeMismatch,
    ParseError,
    ValueOutOfRange,
    InvalidAddress,
    InvalidSymbol,
    InvalidBytes,
    MissingRequiredParam,
    UnknownType,
}

/// Validation warning (non-fatal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: WarningCode,
    pub message: String,
    pub field: Option<String>,
}

impl ValidationWarning {
    pub fn implicit_conversion(param_name: &str, from: &str, to: &str) -> Self {
        Self {
            code: WarningCode::ImplicitConversion,
            message: format!(
                "Parameter '{}': implicit conversion from '{}' to '{}'",
                param_name, from, to
            ),
            field: Some(param_name.to_string()),
        }
    }

    pub fn potential_overflow(param_name: &str) -> Self {
        Self {
            code: WarningCode::PotentialOverflow,
            message: format!(
                "Parameter '{}': value is close to type limits, potential overflow risk",
                param_name
            ),
            field: Some(param_name.to_string()),
        }
    }

    pub fn mutable_call() -> Self {
        Self {
            code: WarningCode::MutableCall,
            message: "This function modifies contract state".to_string(),
            field: None,
        }
    }
}

/// Warning codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    ImplicitConversion,
    PotentialOverflow,
    MutableCall,
    LargeValue,
    DeprecatedFunction,
}

/// Successfully parsed parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedParam {
    pub name: String,
    pub expected_type: String,
    pub value: ParsedValue,
}

/// Request to validate a contract call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCallRequest {
    pub method_name: String,
    pub params: Vec<String>,
    #[serde(default)]
    pub strict: bool,
}

/// Contract call validator
pub struct CallValidator {
    abi: ContractABI,
    strict_mode: bool,
}

impl CallValidator {
    /// Create a new validator with the given ABI
    pub fn new(abi: ContractABI) -> Self {
        Self {
            abi,
            strict_mode: false,
        }
    }

    /// Enable strict mode (no implicit conversions)
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Validate a function call
    pub fn validate_call(&self, method_name: &str, params: &[String]) -> ValidationResult {
        // 1. Check if function exists
        let function = match self.abi.find_function(method_name) {
            Some(f) => f,
            None => {
                return ValidationResult::failure(
                    method_name.to_string(),
                    vec![ValidationError::function_not_found(method_name)],
                );
            }
        };

        // 2. Check function visibility
        if function.visibility != FunctionVisibility::Public {
            return ValidationResult::failure(
                method_name.to_string(),
                vec![ValidationError::function_not_public(method_name)],
            );
        }

        // 3. Check parameter count
        if params.len() != function.params.len() {
            return ValidationResult::failure(
                method_name.to_string(),
                vec![ValidationError::param_count_mismatch(
                    function.params.len(),
                    params.len(),
                )],
            );
        }

        // 4. Validate each parameter
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut parsed_params = Vec::new();

        for (i, (param_value, param_spec)) in params.iter().zip(function.params.iter()).enumerate()
        {
            match self.validate_param(param_value, param_spec, i) {
                Ok((parsed, param_warnings)) => {
                    parsed_params.push(parsed);
                    warnings.extend(param_warnings);
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }

        if !errors.is_empty() {
            return ValidationResult::failure(method_name.to_string(), errors);
        }

        // 5. Add mutable call warning if applicable
        if function.is_mutable {
            warnings.push(ValidationWarning::mutable_call());
        }

        let mut result = ValidationResult::success(
            method_name.to_string(),
            parsed_params,
            &function.return_type,
        );

        for warning in warnings {
            result = result.with_warning(warning);
        }

        result
    }

    /// Validate a single parameter
    fn validate_param(
        &self,
        value: &str,
        spec: &FunctionParam,
        _index: usize,
    ) -> Result<(ParsedParam, Vec<ValidationWarning>), ValidationError> {
        let mut warnings = Vec::new();

        // Parse the value according to expected type
        let parsed = parse_value_string(value, &spec.param_type)
            .map_err(|e| ValidationError::parse_error(&spec.name, &e.message))?;

        // Type checking
        let inferred_type = parsed.infer_type();

        if !self.types_compatible(&inferred_type, &spec.param_type) {
            if self.strict_mode {
                return Err(ValidationError::type_mismatch(
                    &spec.name,
                    &spec.param_type.display_name(),
                    &inferred_type.display_name(),
                ));
            } else if self.can_implicit_convert(&inferred_type, &spec.param_type) {
                warnings.push(ValidationWarning::implicit_conversion(
                    &spec.name,
                    &inferred_type.display_name(),
                    &spec.param_type.display_name(),
                ));
            } else {
                return Err(ValidationError::type_mismatch(
                    &spec.name,
                    &spec.param_type.display_name(),
                    &inferred_type.display_name(),
                ));
            }
        }

        // Range checking for numeric types
        if let Some(range_warning) = self.check_numeric_range(&parsed, &spec.param_type, &spec.name)
        {
            if self.strict_mode {
                return Err(ValidationError::value_out_of_range(
                    &spec.name,
                    &spec.param_type.display_name(),
                ));
            }
            warnings.push(range_warning);
        }

        Ok((
            ParsedParam {
                name: spec.name.clone(),
                expected_type: spec.param_type.display_name(),
                value: parsed,
            },
            warnings,
        ))
    }

    /// Check if two types are compatible
    #[allow(clippy::only_used_in_recursion)]
    fn types_compatible(&self, actual: &SorobanType, expected: &SorobanType) -> bool {
        match (actual, expected) {
            // Exact match
            (a, b) if a == b => true,

            // Integer types: allow same signedness with smaller size
            (SorobanType::I32, SorobanType::I64 | SorobanType::I128 | SorobanType::I256) => true,
            (SorobanType::I64, SorobanType::I128 | SorobanType::I256) => true,
            (SorobanType::I128, SorobanType::I256) => true,

            (SorobanType::U32, SorobanType::U64 | SorobanType::U128 | SorobanType::U256) => true,
            (SorobanType::U64, SorobanType::U128 | SorobanType::U256) => true,
            (SorobanType::U128, SorobanType::U256) => true,

            // String and Symbol are often interchangeable in simple cases
            (SorobanType::String, SorobanType::Symbol) => true,
            (SorobanType::Symbol, SorobanType::String) => true,

            // Option: inner type must be compatible
            (SorobanType::Option { value_type: a }, SorobanType::Option { value_type: b }) => {
                self.types_compatible(a, b)
            }

            // Vec: element type must be compatible
            (SorobanType::Vec { element_type: a }, SorobanType::Vec { element_type: b }) => {
                self.types_compatible(a, b)
            }

            // Custom types: name must match
            (SorobanType::Custom { name: a }, SorobanType::Custom { name: b }) => a == b,

            _ => false,
        }
    }

    /// Check if implicit conversion is allowed
    fn can_implicit_convert(&self, from: &SorobanType, to: &SorobanType) -> bool {
        match (from, to) {
            // Allow widening integer conversions
            (SorobanType::I32, SorobanType::I64 | SorobanType::I128) => true,
            (SorobanType::I64, SorobanType::I128) => true,
            (SorobanType::U32, SorobanType::U64 | SorobanType::U128) => true,
            (SorobanType::U64, SorobanType::U128) => true,

            // Allow signed to larger unsigned if value is positive (checked at runtime)
            // This is implicit conversion, actual value check happens elsewhere
            _ => false,
        }
    }

    /// Check numeric value range
    fn check_numeric_range(
        &self,
        value: &ParsedValue,
        expected_type: &SorobanType,
        param_name: &str,
    ) -> Option<ValidationWarning> {
        match (value, expected_type) {
            (ParsedValue::Integer(n), SorobanType::I32) => {
                let limit = i32::MAX as i128;
                let min_limit = i32::MIN as i128;
                if *n > (limit * 9 / 10) || *n < (min_limit * 9 / 10) {
                    Some(ValidationWarning::potential_overflow(param_name))
                } else {
                    None
                }
            }
            (ParsedValue::Integer(n), SorobanType::I64) => {
                if *n > i64::MAX as i128 || *n < i64::MIN as i128 {
                    Some(ValidationWarning::potential_overflow(param_name))
                } else {
                    None
                }
            }
            (ParsedValue::UnsignedInteger(n), SorobanType::U32) => {
                if *n > u32::MAX as u128 {
                    Some(ValidationWarning::potential_overflow(param_name))
                } else {
                    None
                }
            }
            (ParsedValue::UnsignedInteger(n), SorobanType::U64) => {
                if *n > u64::MAX as u128 {
                    Some(ValidationWarning::potential_overflow(param_name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get information about a function
    pub fn get_function_info(&self, method_name: &str) -> Option<FunctionInfo> {
        self.abi.find_function(method_name).map(|f| FunctionInfo {
            name: f.name.clone(),
            visibility: f.visibility.clone(),
            params: f
                .params
                .iter()
                .map(|p| ParamInfo {
                    name: p.name.clone(),
                    type_name: p.param_type.display_name(),
                    doc: p.doc.clone(),
                })
                .collect(),
            return_type: f.return_type.display_name(),
            doc: f.doc.clone(),
            is_mutable: f.is_mutable,
        })
    }

    /// List all available functions
    pub fn list_functions(&self) -> Vec<FunctionInfo> {
        self.abi
            .functions
            .iter()
            .map(|f| FunctionInfo {
                name: f.name.clone(),
                visibility: f.visibility.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| ParamInfo {
                        name: p.name.clone(),
                        type_name: p.param_type.display_name(),
                        doc: p.doc.clone(),
                    })
                    .collect(),
                return_type: f.return_type.display_name(),
                doc: f.doc.clone(),
                is_mutable: f.is_mutable,
            })
            .collect()
    }
}

/// Function information for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub visibility: FunctionVisibility,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
    pub doc: Option<String>,
    pub is_mutable: bool,
}

/// Parameter information for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub type_name: String,
    pub doc: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_abi() -> ContractABI {
        let mut abi = ContractABI::new("TestToken".to_string());

        abi.functions.push(ContractFunction {
            name: "transfer".to_string(),
            visibility: FunctionVisibility::Public,
            params: vec![
                FunctionParam {
                    name: "to".to_string(),
                    param_type: SorobanType::Address,
                    doc: Some("Recipient address".to_string()),
                },
                FunctionParam {
                    name: "amount".to_string(),
                    param_type: SorobanType::I128,
                    doc: Some("Amount to transfer".to_string()),
                },
            ],
            return_type: SorobanType::Bool,
            doc: Some("Transfer tokens to another address".to_string()),
            is_mutable: true,
        });

        abi.functions.push(ContractFunction {
            name: "balance".to_string(),
            visibility: FunctionVisibility::Public,
            params: vec![FunctionParam {
                name: "address".to_string(),
                param_type: SorobanType::Address,
                doc: Some("Address to check".to_string()),
            }],
            return_type: SorobanType::I128,
            doc: Some("Get balance of an address".to_string()),
            is_mutable: false,
        });

        abi.functions.push(ContractFunction {
            name: "_internal_mint".to_string(),
            visibility: FunctionVisibility::Internal,
            params: vec![],
            return_type: SorobanType::Void,
            doc: None,
            is_mutable: true,
        });

        abi
    }

    #[test]
    fn test_validate_valid_call() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let result = validator.validate_call(
            "transfer",
            &[
                "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
                "1000".to_string(),
            ],
        );

        assert!(result.valid);
        assert_eq!(result.function_name, "transfer");
        assert!(result.parsed_params.is_some());
        assert_eq!(result.parsed_params.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_validate_function_not_found() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let result = validator.validate_call("nonexistent", &[]);

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, ErrorCode::FunctionNotFound);
    }

    #[test]
    fn test_validate_internal_function() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let result = validator.validate_call("_internal_mint", &[]);

        assert!(!result.valid);
        assert_eq!(result.errors[0].code, ErrorCode::FunctionNotPublic);
    }

    #[test]
    fn test_validate_param_count_mismatch() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let result = validator.validate_call("transfer", &["arg1".to_string()]);

        assert!(!result.valid);
        assert_eq!(result.errors[0].code, ErrorCode::ParamCountMismatch);
    }

    #[test]
    fn test_validate_invalid_address() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let result = validator.validate_call(
            "transfer",
            &["invalid_address".to_string(), "1000".to_string()],
        );

        assert!(!result.valid);
        assert_eq!(result.errors[0].code, ErrorCode::ParseError);
    }

    #[test]
    fn test_strict_mode() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi).strict();

        // In strict mode, warnings become errors
        let result = validator.validate_call(
            "balance",
            &["GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string()],
        );

        assert!(result.valid);
    }

    #[test]
    fn test_mutable_call_warning() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let result = validator.validate_call(
            "transfer",
            &[
                "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
                "100".to_string(),
            ],
        );

        assert!(result.valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::MutableCall));
    }

    #[test]
    fn test_list_functions() {
        let abi = create_test_abi();
        let validator = CallValidator::new(abi);

        let functions = validator.list_functions();
        assert_eq!(functions.len(), 3);
        assert!(functions.iter().any(|f| f.name == "transfer"));
        assert!(functions.iter().any(|f| f.name == "balance"));
    }
}
