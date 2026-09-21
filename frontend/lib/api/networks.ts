/**
 * Network endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { NetworkListResponse } from "@/types";

export async function fetchNetworks(): Promise<NetworkListResponse> {
  if (USE_MOCKS) {
    return { networks: [], cached_at: new Date().toISOString() };
  }
  return apiFetch<NetworkListResponse>("/api/networks");
}
