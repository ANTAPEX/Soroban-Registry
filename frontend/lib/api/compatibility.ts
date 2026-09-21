/**
 * Compatibility testing endpoints.
 */

import { apiFetch } from "./client";
import { API_URL, USE_MOCKS } from "@/lib/env";
import type {
  CompatibilityHistoryResponse,
  CompatibilityNotification,
  CompatibilityTestEntry,
  CompatibilityTestMatrixResponse,
  RunCompatibilityTestRequest,
} from "@/types";

export async function fetchCompatibilityMatrix(
  contractId: string,
): Promise<CompatibilityTestMatrixResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      sdk_versions: [],
      wasm_runtimes: [],
      networks: [],
      entries: [],
      summary: {
        total_tests: 0,
        compatible_count: 0,
        warning_count: 0,
        incompatible_count: 0,
      },
      last_tested: null,
    };
  }
  return apiFetch<CompatibilityTestMatrixResponse>(`/api/contracts/${contractId}/compatibility-matrix`);
}

export async function runCompatibilityTest(
  contractId: string,
  request: RunCompatibilityTestRequest,
): Promise<CompatibilityTestEntry> {
  return apiFetch<CompatibilityTestEntry>(`/api/contracts/${contractId}/compatibility-matrix/test`, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export async function fetchCompatibilityHistory(
  contractId: string,
  limit?: number,
  offset?: number,
): Promise<CompatibilityHistoryResponse> {
  if (USE_MOCKS) {
    return { contract_id: contractId, changes: [], total: 0 };
  }
  const search = new URLSearchParams();
  if (limit != null) search.set("limit", String(limit));
  if (offset != null) search.set("offset", String(offset));
  const qs = search.toString() ? `?${search.toString()}` : "";
  return apiFetch<CompatibilityHistoryResponse>(`/api/contracts/${contractId}/compatibility-matrix/history${qs}`);
}

export async function fetchCompatibilityNotifications(
  contractId: string,
): Promise<CompatibilityNotification[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<CompatibilityNotification[]>(`/api/contracts/${contractId}/compatibility-matrix/notifications`);
}

export function getCompatibilityExportUrl(
  contractId: string,
  format: "csv" | "json",
): string {
  return `${API_URL}/api/contracts/${contractId}/compatibility-matrix/export?format=${format}`;
}
