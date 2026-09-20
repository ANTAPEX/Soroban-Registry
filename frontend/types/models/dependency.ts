/**
 * Package dependency and vulnerability-scan types for Soroban Registry
 */

export interface PackageDependency {
  id: string;
  contract_id: string;
  package_name: string;
  version: string;
  created_at: string;
}

export interface PackageDependencyInput {
  package_name: string;
  version: string;
}

export interface DependencyVulnerabilityFinding {
  package_name: string;
  version: string;
  cve_id: string;
  severity: string;
  description?: string | null;
  recommended_version?: string | null;
}

export type DependencyScanStatus = "not_scanned" | "clean" | "vulnerable";

export interface DependencyScanReport {
  contract_id: string;
  status: DependencyScanStatus;
  dependencies_scanned: number;
  vulnerable_dependency_count: number;
  last_scanned_at?: string | null;
  findings: DependencyVulnerabilityFinding[];
}
