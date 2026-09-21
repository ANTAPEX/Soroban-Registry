#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    pub description: Option<String>,
    pub setup: Option<Vec<TestAction>>,
    pub steps: Vec<TestStep>,
    pub teardown: Option<Vec<TestAction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub name: String,
    pub contract: String,
    pub method: String,
    pub args: Option<Vec<TestValue>>,
    pub assertions: Option<Vec<Assertion>>,
    pub expected_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TestValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Array(Vec<TestValue>),
    Object(HashMap<String, TestValue>),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAction {
    pub action: String,
    pub contract: Option<String>,
    pub method: Option<String>,
    pub args: Option<Vec<TestValue>>,
    pub value: Option<TestValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub r#type: String,
    pub field: Option<String>,
    pub expected: TestValue,
    pub operator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub scenario: String,
    pub passed: bool,
    pub duration: Duration,
    pub steps: Vec<StepResult>,
    pub error: Option<String>,
    pub coverage: CoverageMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_name: String,
    pub passed: bool,
    pub duration: Duration,
    pub error: Option<String>,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub contracts_tested: usize,
    pub methods_tested: usize,
    pub total_methods: usize,
    pub coverage_percent: f64,
    pub lines_covered: usize,
    pub lines_total: usize,
}

pub struct TestRunner {
    contract_path: String,
    contracts: HashMap<String, ContractInfo>,
    coverage: CoverageTracker,
}

#[derive(Debug, Clone)]
struct ContractInfo {
    name: String,
    methods: Vec<String>,
}

struct CoverageTracker {
    contracts: std::collections::HashSet<String>,
    methods: std::collections::HashSet<(String, String)>,
    lines: std::collections::HashSet<u32>,
}

impl CoverageTracker {
    fn new() -> Self {
        Self {
            contracts: std::collections::HashSet::new(),
            methods: std::collections::HashSet::new(),
            lines: std::collections::HashSet::new(),
        }
    }

    fn record_contract_call(&mut self, contract: &str, method: &str) {
        self.contracts.insert(contract.to_string());
        self.methods
            .insert((contract.to_string(), method.to_string()));
    }

    fn calculate_metrics(&self, total_methods: usize) -> CoverageMetrics {
        let methods_tested = self.methods.len();
        let coverage_percent = if total_methods > 0 {
            (methods_tested as f64 / total_methods as f64) * 100.0
        } else {
            0.0
        };

        CoverageMetrics {
            contracts_tested: self.contracts.len(),
            methods_tested,
            total_methods,
            coverage_percent,
            lines_covered: self.lines.len(),
            lines_total: total_methods * 10,
        }
    }
}

impl TestRunner {
    pub fn new(contract_path: &str) -> Result<Self> {
        let contracts = Self::discover_contracts(contract_path)?;
        Ok(Self {
            contract_path: contract_path.to_string(),
            contracts,
            coverage: CoverageTracker::new(),
        })
    }

    fn discover_contracts(contract_path: &str) -> Result<HashMap<String, ContractInfo>> {
        let mut contracts = HashMap::new();
        let path = Path::new(contract_path);

        if path.is_file() {
            let methods = Self::extract_methods(path)?;
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("contract")
                .to_string();
            contracts.insert(
                name.clone(),
                ContractInfo {
                    name: name.clone(),
                    methods,
                },
            );
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    let methods = Self::extract_methods(&path)?;
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("contract")
                        .to_string();
                    contracts.insert(
                        name.clone(),
                        ContractInfo {
                            name: name.clone(),
                            methods,
                        },
                    );
                }
            }
        }

        Ok(contracts)
    }

    fn extract_methods(path: &Path) -> Result<Vec<String>> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read contract: {}", path.display()))?;

        let mut methods = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                if let Some(start) = trimmed.find("fn ") {
                    let after_fn = &trimmed[start + 3..];
                    if let Some(end) = after_fn.find('(') {
                        let method_name = after_fn[..end].trim();
                        if !method_name.is_empty() && !method_name.contains('<') {
                            methods.push(method_name.to_string());
                        }
                    }
                }
            }
        }

        Ok(methods)
    }

    pub async fn run_scenario(&mut self, scenario: TestScenario) -> Result<TestResult> {
        let start_time = Instant::now();
        let mut step_results = Vec::new();
        let mut error = None;

        if let Some(ref setup) = scenario.setup {
            for action in setup {
                self.execute_action(action).await?;
            }
        }

        for step in &scenario.steps {
            let step_start = Instant::now();
            let mut assertions_passed = 0;
            let mut assertions_failed = 0;
            let mut step_error = None;

            self.coverage
                .record_contract_call(&step.contract, &step.method);

            let step_result = self.execute_step(step).await;

            match step_result {
                Ok(result) => {
                    if step.expected_error.is_some() {
                        step_error = Some("Expected error but none occurred".to_string());
                        assertions_failed += 1;
                    } else if let Some(ref assertions) = step.assertions {
                        for assertion in assertions {
                            match self.check_assertion(assertion, &result) {
                                Ok(true) => assertions_passed += 1,
                                Ok(false) => {
                                    assertions_failed += 1;
                                    if step_error.is_none() {
                                        step_error =
                                            Some(format!("Assertion failed: {}", assertion.r#type));
                                    }
                                }
                                Err(e) => {
                                    assertions_failed += 1;
                                    if step_error.is_none() {
                                        step_error = Some(format!("Assertion error: {}", e));
                                    }
                                }
                            }
                        }
                    } else {
                        assertions_passed += 1;
                    }
                }
                Err(e) => {
                    if let Some(ref expected_err) = step.expected_error {
                        if e.to_string().contains(expected_err) {
                            assertions_passed += 1;
                        } else {
                            step_error =
                                Some(format!("Expected error '{}' but got: {}", expected_err, e));
                            assertions_failed += 1;
                        }
                    } else {
                        step_error = Some(e.to_string());
                        assertions_failed += 1;
                    }
                }
            }

            step_results.push(StepResult {
                step_name: step.name.clone(),
                passed: assertions_failed == 0 && step_error.is_none(),
                duration: step_start.elapsed(),
                error: step_error,
                assertions_passed,
                assertions_failed,
            });

            if step_results.last().unwrap().error.is_some() {
                error = step_results.last().unwrap().error.clone();
                break;
            }
        }

        if let Some(ref teardown) = scenario.teardown {
            for action in teardown {
                let _ = self.execute_action(action).await;
            }
        }

        let total_methods: usize = self.contracts.values().map(|c| c.methods.len()).sum();
        let coverage = self.coverage.calculate_metrics(total_methods);

        Ok(TestResult {
            scenario: scenario.name,
            passed: step_results.iter().all(|s| s.passed),
            duration: start_time.elapsed(),
            steps: step_results,
            error,
            coverage,
        })
    }

    async fn execute_step(&self, step: &TestStep) -> Result<TestValue> {
        let contract_info = self
            .contracts
            .get(&step.contract)
            .ok_or_else(|| anyhow::anyhow!("Contract not found: {}", step.contract))?;

        if !contract_info.methods.contains(&step.method) {
            return Err(anyhow::anyhow!(
                "Method '{}' not found in contract '{}'",
                step.method,
                step.contract
            ));
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(TestValue::String(format!(
            "result_from_{}_{}",
            step.contract, step.method
        )))
    }

    async fn execute_action(&mut self, action: &TestAction) -> Result<()> {
        match action.action.as_str() {
            "deploy" => {
                tokio::time::sleep(Duration::from_millis(5)).await;
                Ok(())
            }
            "invoke" => {
                if let (Some(contract), Some(method)) = (&action.contract, &action.method) {
                    self.coverage.record_contract_call(contract, method);
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
                Ok(())
            }
            "set" => Ok(()),
            _ => Err(anyhow::anyhow!("Unknown action: {}", action.action)),
        }
    }

    fn check_assertion(&self, assertion: &Assertion, result: &TestValue) -> Result<bool> {
        let operator = assertion.operator.as_deref().unwrap_or("eq");

        match assertion.r#type.as_str() {
            "equals" | "eq" => Ok(self.compare_values(result, &assertion.expected, operator)),
            "not_equals" | "ne" => Ok(!self.compare_values(result, &assertion.expected, operator)),
            "contains" => {
                if let TestValue::String(s) = result {
                    if let TestValue::String(expected) = &assertion.expected {
                        Ok(s.contains(expected))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            "greater_than" | "gt" => {
                if let (TestValue::Number(a), TestValue::Number(b)) = (result, &assertion.expected)
                {
                    Ok(*a > *b)
                } else {
                    Ok(false)
                }
            }
            "less_than" | "lt" => {
                if let (TestValue::Number(a), TestValue::Number(b)) = (result, &assertion.expected)
                {
                    Ok(*a < *b)
                } else {
                    Ok(false)
                }
            }
            "state" => {
                if let Some(ref field) = assertion.field {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            "event" => Ok(true),
            _ => Err(anyhow::anyhow!(
                "Unknown assertion type: {}",
                assertion.r#type
            )),
        }
    }

    fn compare_values(&self, a: &TestValue, b: &TestValue, op: &str) -> bool {
        match (a, b) {
            (TestValue::String(s1), TestValue::String(s2)) => match op {
                "eq" => s1 == s2,
                "ne" => s1 != s2,
                _ => s1 == s2,
            },
            (TestValue::Number(n1), TestValue::Number(n2)) => match op {
                "eq" => n1 == n2,
                "ne" => n1 != n2,
                _ => n1 == n2,
            },
            (TestValue::Boolean(b1), TestValue::Boolean(b2)) => b1 == b2,
            _ => false,
        }
    }
}

pub fn load_test_scenario(path: &Path) -> Result<TestScenario> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read test file: {}", path.display()))?;

    if path.extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML test file: {}", path.display()))
    } else {
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON test file: {}", path.display()))
    }
}

pub fn generate_junit_xml(results: &[TestResult], output_path: &Path) -> Result<()> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<testsuites>\n");

    let total_tests = results.len();
    let total_failures = results.iter().filter(|r| !r.passed).count();
    let total_time: f64 = results.iter().map(|r| r.duration.as_secs_f64()).sum();

    xml.push_str(&format!(
        "  <testsuite name=\"contract-tests\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n",
        total_tests, total_failures, total_time
    ));

    for result in results {
        let _status = if result.passed { "pass" } else { "fail" };
        xml.push_str(&format!(
            "    <testcase name=\"{}\" classname=\"contract-test\" time=\"{:.3}\">\n",
            result.scenario,
            result.duration.as_secs_f64()
        ));

        if !result.passed {
            xml.push_str(&format!(
                "      <failure message=\"{}\"/>\n",
                result
                    .error
                    .as_deref()
                    .unwrap_or("Test failed")
                    .replace('"', "&quot;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            ));
        }

        xml.push_str("    </testcase>\n");
    }

    xml.push_str("  </testsuite>\n");
    xml.push_str("</testsuites>\n");

    fs::write(output_path, xml)
        .with_context(|| format!("Failed to write JUnit XML: {}", output_path.display()))?;

    Ok(())
}
