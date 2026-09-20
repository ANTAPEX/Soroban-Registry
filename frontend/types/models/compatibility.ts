/**
 * Contract compatibility-testing types for Soroban Registry
 */

export type CompatibilityTestStatus = "compatible" | "warning" | "incompatible";

export interface CompatibilityTestEntry {
  sdk_version: string;
  wasm_runtime: string;
  network: string;
  status: CompatibilityTestStatus;
  tested_at: string;
  test_duration_ms?: number | null;
  error_message?: string | null;
}

export interface CompatibilityHistoryEntry {
  id: string;
  sdk_version: string;
  wasm_runtime: string;
  network: string;
  previous_status?: CompatibilityTestStatus | null;
  new_status: CompatibilityTestStatus;
  changed_at: string;
  change_reason?: string | null;
}

export interface CompatibilityTestSummary {
  total_tests: number;
  compatible_count: number;
  warning_count: number;
  incompatible_count: number;
}

export interface CompatibilityTestMatrixResponse {
  contract_id: string;
  sdk_versions: string[];
  wasm_runtimes: string[];
  networks: string[];
  entries: CompatibilityTestEntry[];
  summary: CompatibilityTestSummary;
  last_tested?: string | null;
}

export interface RunCompatibilityTestRequest {
  sdk_version: string;
  wasm_runtime: string;
  network: string;
}

export interface CompatibilityHistoryResponse {
  contract_id: string;
  changes: CompatibilityHistoryEntry[];
  total: number;
}

export interface CompatibilityNotification {
  id: string;
  contract_id: string;
  sdk_version: string;
  message: string;
  is_read: boolean;
  created_at: string;
}
