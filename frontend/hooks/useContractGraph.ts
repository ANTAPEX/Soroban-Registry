"use client";

import { useCallback, useMemo } from "react";
import { useContractLocalGraph } from "@/hooks/queries";
import type { GraphEdge, GraphNode } from "@/types";

export interface ContractGraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

const EMPTY: ContractGraphData = { nodes: [], edges: [] };

/**
 * A contract's local graph plus the export and counting helpers the 3D view
 * needs, over `useContractLocalGraph`.
 *
 * This used to fetch the endpoint itself, so the 3D view and the interaction
 * flow held separate copies of the same graph. Both now read the same cache,
 * and share an entry whenever they ask for the same contract and depth.
 */
export function useContractGraph(contractId: string, depth = 2) {
  const query = useContractLocalGraph(contractId, depth, {
    enabled: !!contractId,
  });

  const graph = useMemo<ContractGraphData>(
    () =>
      query.data
        ? { nodes: query.data.nodes, edges: query.data.edges }
        : EMPTY,
    [query.data],
  );

  const exportAsJson = useCallback(
    () => JSON.stringify(graph, null, 2),
    [graph],
  );

  const stats = useMemo(
    () => ({ nodeCount: graph.nodes.length, edgeCount: graph.edges.length }),
    [graph.edges.length, graph.nodes.length],
  );

  const refresh = useCallback(async () => {
    await query.refetch();
  }, [query]);

  return {
    graph,
    isLoading: query.isPending,
    error: query.error ? query.error.message : null,
    stats,
    refresh,
    exportAsJson,
  };
}
