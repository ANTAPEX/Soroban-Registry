/**
 * Contract interoperability-analysis types for Soroban Registry
 */

import type { Network } from "./network";
import type { GraphResponse } from "./graph";

export type InteroperabilityCapabilityKind = "bridge" | "adapter";

export interface InteroperabilityProtocolMatch {
  slug: string;
  name: string;
  description: string;
  status: "compliant" | "partial" | "unsupported";
  matched_functions: string[];
  missing_functions: string[];
  optional_matches: string[];
  compliance_score: number;
}

export interface InteroperabilityCapability {
  kind: InteroperabilityCapabilityKind;
  label: string;
  confidence: number;
  evidence: string[];
}

export interface InteroperabilitySuggestion {
  contract_id: string;
  contract_address: string;
  contract_name: string;
  network: Network;
  category?: string | null;
  is_verified: boolean;
  score: number;
  reason: string;
  shared_protocols: string[];
  shared_functions: string[];
  relation_types: string[];
}

export interface InteroperabilitySummary {
  protocol_matches: number;
  compatible_contracts: number;
  suggested_contracts: number;
  graph_nodes: number;
  graph_edges: number;
  bridge_signals: number;
  adapter_signals: number;
}

export interface ContractInteroperabilityResponse {
  contract_id: string;
  contract_address: string;
  contract_name: string;
  network: Network;
  analyzed_at: string;
  has_abi: boolean;
  analyzed_functions: string[];
  warnings: string[];
  protocols: InteroperabilityProtocolMatch[];
  capabilities: InteroperabilityCapability[];
  suggestions: InteroperabilitySuggestion[];
  graph: GraphResponse;
  summary: InteroperabilitySummary;
}
