/**
 * Dependency-graph types for Soroban Registry
 */

import type { Network } from "./network";

export interface GraphNode {
  id: string;
  contract_id: string;
  name: string;
  network: Network;
  is_verified: boolean;
  category?: string | null;
  tags: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
  dependency_type: string;
  call_frequency?: number | null;
  call_volume?: number | null;
  is_estimated?: boolean;
  is_circular?: boolean;
}

export interface GraphResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
}
