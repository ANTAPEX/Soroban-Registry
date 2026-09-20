/**
 * SDK compatibility queries, over `lib/api/compatibility.ts`.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchCompatibilityMatrix,
  fetchCompatibilityHistory,
  fetchCompatibilityNotifications,
  runCompatibilityTest,
} from "@/lib/api/compatibility";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type {
  CompatibilityHistoryResponse,
  CompatibilityNotification,
  CompatibilityTestEntry,
  CompatibilityTestMatrixResponse,
  RunCompatibilityTestRequest,
} from "@/types";

export function useCompatibilityMatrix(
  contractId: string,
  options?: QueryOpts<CompatibilityTestMatrixResponse>,
) {
  return useQuery({
    queryKey: queryKeys.compatibilityMatrix(contractId),
    queryFn: () => fetchCompatibilityMatrix(contractId),
    enabled: !!contractId,
    ...options,
  });
}

export function useCompatibilityHistory(
  contractId: string,
  limit?: number,
  options?: QueryOpts<CompatibilityHistoryResponse>,
) {
  return useQuery({
    queryKey: queryKeys.compatibilityHistory(contractId),
    queryFn: () => fetchCompatibilityHistory(contractId, limit),
    enabled: !!contractId,
    ...options,
  });
}

export function useCompatibilityNotifications(
  contractId: string,
  options?: QueryOpts<CompatibilityNotification[]>,
) {
  return useQuery({
    queryKey: queryKeys.compatibilityNotifications(contractId),
    queryFn: () => fetchCompatibilityNotifications(contractId),
    enabled: !!contractId,
    ...options,
  });
}

/** A run invalidates all three compatibility views it can change. */
export function useRunCompatibilityTest(
  contractId: string,
  options?: MutationOpts<CompatibilityTestEntry, RunCompatibilityTestRequest>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: RunCompatibilityTestRequest) =>
      runCompatibilityTest(contractId, request),
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.compatibilityMatrix(contractId),
      });
      queryClient.invalidateQueries({
        queryKey: queryKeys.compatibilityHistory(contractId),
      });
      queryClient.invalidateQueries({
        queryKey: queryKeys.compatibilityNotifications(contractId),
      });
      options?.onSuccess?.(...args);
    },
  });
}
