/**
 * Contract graph queries, over `lib/api/graph.ts`.
 */

import { useQuery } from "@tanstack/react-query";
import {
  fetchContractGraph,
  fetchContractLocalGraph,
} from "@/lib/api/graph";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type { GraphResponse, Network } from "@/types";

export function useContractGraph(
  network?: Network | string,
  options?: QueryOpts<GraphResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractGraph(network),
    queryFn: () => fetchContractGraph(network || undefined),
    ...options,
  });
}

export function useContractLocalGraph(
  contractId: string,
  depth?: number,
  options?: QueryOpts<GraphResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractLocalGraph(contractId, depth),
    queryFn: () => fetchContractLocalGraph(contractId, depth),
    ...options,
  });
}
