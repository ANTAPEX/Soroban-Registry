/**
 * Legacy aggregate stats types for Soroban Registry
 */

import type { StatsResponse } from "../stats";
export interface LegacyStatsResponse extends StatsResponse {
  total_contracts: number;
  verified_contracts: number;
  total_publishers: number;
}
