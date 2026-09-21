/**
 * Dependency endpoints: the dependency tree, vulnerability scanning
 * and declared package dependencies.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type {
  DependencyScanReport,
  DependencyTreeNode,
  PackageDependency,
  PackageDependencyInput,
} from "@/types";

export async function fetchDependencyTree(id: string): Promise<DependencyTreeNode> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      name: id,
      current_version: "1.0.0",
      constraint_to_parent: "",
      dependencies: [],
    };
  }
  return apiFetch<DependencyTreeNode>(`/api/contracts/${id}/dependencies/tree`);
}

export async function fetchDependencyScanReport(
  contractId: string,
): Promise<DependencyScanReport> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      status: "not_scanned",
      dependencies_scanned: 0,
      vulnerable_dependency_count: 0,
      last_scanned_at: null,
      findings: [],
    };
  }
  return apiFetch<DependencyScanReport>(
    `/api/contracts/${contractId}/dependency-scan`,
  );
}

export async function triggerDependencyScan(
  contractId: string,
): Promise<DependencyScanReport> {
  if (USE_MOCKS) {
    return fetchDependencyScanReport(contractId);
  }
  return apiFetch<DependencyScanReport>(
    `/api/contracts/${contractId}/dependency-scan`,
    { method: "POST" },
  );
}

export async function fetchPackageDependencies(
  contractId: string,
): Promise<PackageDependency[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<PackageDependency[]>(
    `/api/contracts/${contractId}/package-dependencies`,
  );
}

export async function declarePackageDependencies(
  contractId: string,
  dependencies: PackageDependencyInput[],
): Promise<DependencyScanReport> {
  return apiFetch<DependencyScanReport>(
    `/api/contracts/${contractId}/package-dependencies`,
    {
      method: "POST",
      body: JSON.stringify({ dependencies }),
    },
  );
}
