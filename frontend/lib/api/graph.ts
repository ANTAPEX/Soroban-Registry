/**
 * Contract graph endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { GraphResponse, Network } from "@/types";

export async function fetchContractGraph(network?: Network | string): Promise<GraphResponse> {
  if (USE_MOCKS) {
    return { nodes: [], edges: [] };
  }
  const qs = network ? `?network=${network}` : "";
  return apiFetch<GraphResponse>(`/api/contracts/graph${qs}`);
}

export async function fetchContractLocalGraph(
  contractId: string,
  depth?: number,
): Promise<GraphResponse> {
  if (USE_MOCKS) {
    return { nodes: [], edges: [] };
  }
  const search = new URLSearchParams();
  if (depth != null) search.set("depth", String(depth));
  const qs = search.toString() ? `?${search.toString()}` : "";
  return apiFetch<GraphResponse>(`/api/contracts/${contractId}/graph${qs}`);
}
