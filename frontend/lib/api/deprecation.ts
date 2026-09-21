/**
 * Deprecation endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { DeprecationInfo } from "@/types";

export async function fetchDeprecationInfo(contractId: string): Promise<DeprecationInfo> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      status: "active",
      dependents_notified: 0,
    };
  }
  return apiFetch<DeprecationInfo>(`/api/contracts/${contractId}/deprecation`);
}

export async function setDeprecation(
  contractId: string,
  data: Partial<DeprecationInfo>,
): Promise<DeprecationInfo> {
  return apiFetch<DeprecationInfo>(`/api/contracts/${contractId}/deprecation`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}
