use crate::simulation::abi_extractor::AbiExtractionResult;
use crate::simulation::wasm_validator::WasmValidationResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysisResult {
    pub estimated_execution_time_ms: u64,
    pub memory_estimate_kb: u64,
    pub warnings: Vec<PerformanceWarning>,
    pub function_analysis: Vec<FunctionAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceWarning {
    pub code: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAnalysis {
    pub name: String,
    pub complexity: String,
    pub recommendation: Option<String>,
}

pub fn analyze_performance(
    wasm_bytes: &[u8],
    validation_result: &WasmValidationResult,
    abi_result: &AbiExtractionResult,
) -> PerformanceAnalysisResult {
    let mut warnings = Vec::new();
    let mut function_analysis = Vec::new();

    // Estimate execution time based on WASM size and complexity
    let base_time_per_kb = 1u64; // 1ms per KB as baseline
    let wasm_size_kb = wasm_bytes.len() as u64 / 1024;
    let estimated_execution_time_ms = base_time_per_kb * wasm_size_kb.max(1);

    // Memory estimation based on memory pages
    let memory_estimate_kb = validation_result.memory_pages * 64; // 64KB per page

    // Check for potential performance issues

    // Large WASM size warning
    if wasm_size_kb > 100 {
        warnings.push(PerformanceWarning {
            code: "LARGE_WASM".to_string(),
            message: format!("WASM size is {} KB - consider optimizing", wasm_size_kb),
            severity: "medium".to_string(),
        });
    }

    // High memory usage warning
    if validation_result.memory_pages > 512 {
        warnings.push(PerformanceWarning {
            code: "HIGH_MEMORY".to_string(),
            message: format!(
                "Memory allocation is {} pages - may exceed typical limits",
                validation_result.memory_pages
            ),
            severity: "high".to_string(),
        });
    }

    // Many tables warning
    if validation_result.table_count > 10 {
        warnings.push(PerformanceWarning {
            code: "MANY_TABLES".to_string(),
            message: format!(
                "{} tables detected - may impact performance",
                validation_result.table_count
            ),
            severity: "low".to_string(),
        });
    }

    // Analyze each exported function
    for func_name in &validation_result.export_functions {
        let analysis = analyze_function(func_name, abi_result);
        function_analysis.push(analysis);
    }

    // Check for unused import warnings
    if !validation_result.import_functions.is_empty() {
        let unused_count = validation_result.import_functions.len();
        if unused_count > 5 {
            warnings.push(PerformanceWarning {
                code: "MANY_IMPORTS".to_string(),
                message: format!("{} imported functions - consider bundling", unused_count),
                severity: "low".to_string(),
            });
        }
    }

    // Data section warning
    if validation_result.data_section_size > 50 {
        warnings.push(PerformanceWarning {
            code: "LARGE_DATA".to_string(),
            message: format!(
                "Large data section ({} entries) - consider lazy loading",
                validation_result.data_section_size
            ),
            severity: "medium".to_string(),
        });
    }

    PerformanceAnalysisResult {
        estimated_execution_time_ms,
        memory_estimate_kb,
        warnings,
        function_analysis,
    }
}

fn analyze_function(func_name: &str, abi_result: &AbiExtractionResult) -> FunctionAnalysis {
    // Check if function is in ABI result
    let has_abi = abi_result.functions.iter().any(|f| f.name == func_name);

    let (complexity, recommendation) =
        if func_name.starts_with("get_") || func_name.contains("_view") {
            ("low".to_string(), None)
        } else if func_name.contains("iterate") || func_name.contains("batch") {
            (
                "high".to_string(),
                Some("Consider adding pagination for large datasets".to_string()),
            )
        } else if has_abi {
            let func = abi_result.functions.iter().find(|f| f.name == func_name);
            if let Some(f) = func {
                if f.param_count > 5 {
                    (
                        "medium".to_string(),
                        Some("Consider grouping parameters into structs".to_string()),
                    )
                } else {
                    ("low".to_string(), None)
                }
            } else {
                ("unknown".to_string(), None)
            }
        } else {
            ("unknown".to_string(), None)
        };

    FunctionAnalysis {
        name: func_name.to_string(),
        complexity,
        recommendation,
    }
}
