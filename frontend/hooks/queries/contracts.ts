/**
 * One hook per contract endpoint, over `lib/api/contracts.ts`.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchContracts,
  fetchContract,
  fetchContractHealth,
  fetchContractAnalytics,
  fetchContractVersions,
  fetchContractAbi,
  fetchContractChangelog,
  fetchContractInteractions,
  publishContract,
} from "@/lib/api/contracts";
import { fetchContractExamples } from "@/lib/api/examples";
import { fetchDependencyTree } from "@/lib/api/dependencies";
import { fetchDeprecationInfo } from "@/lib/api/deprecation";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type {
  Contract,
  ContractAbiResponse,
  ContractAnalyticsResponse,
  ContractChangelogResponse,
  ContractExample,
  ContractGetResponse,
  ContractHealth,
  ContractSearchParams,
  ContractVersion,
  DependencyTreeNode,
  DeprecationInfo,
  InteractionsListResponse,
  InteractionsQueryParams,
  PaginatedResponse,
  PublishRequest,
} from "@/types";

export function useContracts(
  params: ContractSearchParams = {},
  options?: QueryOpts<PaginatedResponse<Contract>>,
) {
  return useQuery({
    queryKey: queryKeys.contractsList(params),
    queryFn: () => fetchContracts(params),
    ...options,
  });
}

export function useRecentContracts(
  limit = 6,
  options?: QueryOpts<PaginatedResponse<Contract>>,
) {
  return useQuery({
    queryKey: queryKeys.contractsRecent(),
    queryFn: () => fetchContracts({ page: 1, page_size: limit }),
    ...options,
  });
}

export function useContract(
  id: string | undefined,
  options?: QueryOpts<ContractGetResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contract(id),
    queryFn: () => fetchContract(id!),
    enabled: !!id,
    ...options,
  });
}

export function useContractHealth(
  id: string,
  options?: QueryOpts<ContractHealth>,
) {
  return useQuery({
    queryKey: queryKeys.contractHealth(id),
    queryFn: () => fetchContractHealth(id),
    ...options,
  });
}

export function useContractAnalytics(
  id: string,
  options?: QueryOpts<ContractAnalyticsResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractAnalytics(id),
    queryFn: () => fetchContractAnalytics(id),
    ...options,
  });
}

/**
 * The same endpoint as `useContractAnalytics`, under the separate cache key the
 * contract detail page has always used. Collapsing the two would change what is
 * cached, so it is left for a follow-up rather than slipped into a move.
 */
export function useContractAnalyticsSummary(
  id: string | undefined,
  options?: QueryOpts<ContractAnalyticsResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractAnalyticsSummary(id),
    queryFn: () => fetchContractAnalytics(id!),
    enabled: !!id,
    ...options,
  });
}

export function useContractVersions(
  id: string | undefined,
  options?: QueryOpts<ContractVersion[]>,
) {
  return useQuery({
    queryKey: queryKeys.contractVersions(id),
    queryFn: () => fetchContractVersions(id!),
    enabled: !!id,
    ...options,
  });
}

export function useContractAbi(
  id: string | undefined,
  version?: string,
  options?: QueryOpts<ContractAbiResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractAbi(id, version),
    queryFn: () => fetchContractAbi(id!, version),
    enabled: !!id,
    ...options,
  });
}

export function useContractChangelog(
  id: string | undefined,
  options?: QueryOpts<ContractChangelogResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractChangelog(id),
    queryFn: () => fetchContractChangelog(id!),
    enabled: !!id,
    ...options,
  });
}

export function useContractInteractions(
  id: string | undefined,
  params?: InteractionsQueryParams,
  options?: QueryOpts<InteractionsListResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractInteractions(id, params),
    queryFn: () => fetchContractInteractions(id!, params),
    enabled: !!id,
    ...options,
  });
}

export function useContractDependencies(
  id: string | undefined,
  options?: QueryOpts<DependencyTreeNode>,
) {
  return useQuery({
    queryKey: queryKeys.contractDependencies(id),
    queryFn: () => fetchDependencyTree(id!),
    enabled: !!id,
    ...options,
  });
}

export function useContractDeprecation(
  id: string | undefined,
  options?: QueryOpts<DeprecationInfo>,
) {
  return useQuery({
    queryKey: queryKeys.contractDeprecation(id),
    queryFn: () => fetchDeprecationInfo(id!),
    enabled: !!id,
    ...options,
  });
}

export function useContractExamples(
  id: string,
  options?: QueryOpts<ContractExample[]>,
) {
  return useQuery({
    queryKey: queryKeys.contractExamples(id),
    queryFn: () => fetchContractExamples(id),
    ...options,
  });
}

export function usePublishContract(
  options?: MutationOpts<Contract, PublishRequest>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: PublishRequest) => publishContract(data),
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.contracts() });
      options?.onSuccess?.(...args);
    },
  });
}
