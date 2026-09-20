/**
 * Formal verification endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { FormalVerificationReport } from "@/types";

export async function fetchFormalVerificationResults(
  contractId: string,
): Promise<FormalVerificationReport[]> {
  if (USE_MOCKS) {
    return [];
  }
  const listResponse = await apiFetch<{
    items: Array<{ id: string }>;
    total: number;
  }>(`/api/contracts/${contractId}/formal-verification`);

  if (!listResponse.items.length) {
    return [];
  }

  const latestSession = listResponse.items[0];
  const detail = await apiFetch<FormalVerificationReport>(
    `/api/contracts/${contractId}/formal-verification/${latestSession.id}`,
  );
  return [detail];
}
