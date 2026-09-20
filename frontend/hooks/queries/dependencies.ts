/**
 * Dependency vulnerability scanning, over `lib/api/dependencies.ts`.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchDependencyScanReport,
  triggerDependencyScan,
} from "@/lib/api/dependencies";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type { DependencyScanReport } from "@/types";

export function useDependencyScan(
  contractId: string,
  options?: QueryOpts<DependencyScanReport>,
) {
  return useQuery({
    queryKey: queryKeys.dependencyScan(contractId),
    queryFn: () => fetchDependencyScanReport(contractId),
    ...options,
  });
}

/**
 * A re-scan returns the fresh report, so it is written into the cache directly
 * rather than invalidated — that is what the panel did before and it saves a
 * round trip.
 */
export function useTriggerDependencyScan(
  contractId: string,
  options?: MutationOpts<DependencyScanReport, void>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => triggerDependencyScan(contractId),
    ...options,
    onSuccess: (data, ...rest) => {
      queryClient.setQueryData(queryKeys.dependencyScan(contractId), data);
      options?.onSuccess?.(data, ...rest);
    },
  });
}
