/**
 * Contract example endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import { MOCK_EXAMPLES } from "./mocks";
import type { ContractExample } from "@/types";

export async function fetchContractExamples(contractId: string): Promise<ContractExample[]> {
  if (USE_MOCKS) {
    return MOCK_EXAMPLES[contractId] || [];
  }
  return apiFetch<ContractExample[]>(`/api/contracts/${contractId}/examples`);
}

export async function rateExample(
  exampleId: string,
  userId: string,
  rating: number,
): Promise<void> {
  await apiFetch<void>(`/api/examples/${exampleId}/rating`, {
    method: "POST",
    body: JSON.stringify({ user_id: userId, rating }),
  });
}
