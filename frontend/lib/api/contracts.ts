/**
 * Contract endpoints: listing, detail, health, analytics, versions,
 * ABI, changelog, recommendations, interactions and publishing.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import { ApiError } from "../errors";
import { MOCK_CONTRACTS, MOCK_VERSIONS } from "./mocks";
import { detectIntent, semanticScore, tokenizeQuery } from "./semanticRanking";
import type {
  Contract,
  ContractAbiResponse,
  ContractAnalyticsResponse,
  ContractChangelogResponse,
  ContractGetResponse,
  ContractHealth,
  ContractRecommendationsResponse,
  ContractSearchParams,
  ContractVersion,
  InteractionsListResponse,
  InteractionsQueryParams,
  Network,
  PaginatedResponse,
  PublishRequest,
  QueryNode,
} from "@/types";

export async function fetchContracts(
  params: ContractSearchParams = {},
): Promise<PaginatedResponse<Contract>> {
  if (USE_MOCKS) {
    const {
      query = "",
      page = 1,
      page_size = 20,
      network,
      networks,
      verified_only,
      category,
      categories,
      tags,
      sort_by,
      sort_order = "desc",
    } = params;

    let results = [...MOCK_CONTRACTS] as Contract[];

    if (query.trim()) {
      const tokens = tokenizeQuery(query);
      const intent = detectIntent(query, params);
      results = results
        .map((c) => ({ ...c, relevance_score: semanticScore(c, tokens, intent) }))
        .filter((c) => (c.relevance_score || 0) > 0.05)
        .sort((a, b) => (b.relevance_score || 0) - (a.relevance_score || 0));
    }

    if (network) results = results.filter((c) => c.network === network);
    if (networks?.length) results = results.filter((c) => networks.includes(c.network));
    if (verified_only) results = results.filter((c) => c.is_verified);
    if (category) results = results.filter((c) => c.category === category);
    if (categories?.length) results = results.filter((c) => categories.includes(c.category ?? ""));
    if (tags && tags.length > 0)
      results = results.filter((c) => tags.some((t) => c.tags?.includes(t)));

    if (sort_by && !query.trim()) {
      results.sort((a, b) => {
        let aVal: number | string = 0;
        let bVal: number | string = 0;
        if (sort_by === "name") { aVal = a.name; bVal = b.name; }
        else if (sort_by === "created_at") { aVal = a.created_at; bVal = b.created_at; }
        else if (sort_by === "updated_at") { aVal = a.updated_at; bVal = b.updated_at; }
        else if (sort_by === "popularity") { aVal = a.popularity_score || 0; bVal = b.popularity_score || 0; }
        else if (sort_by === "deployments") { aVal = a.deployment_count || 0; bVal = b.deployment_count || 0; }
        else if (sort_by === "interactions") { aVal = a.interaction_count || 0; bVal = b.interaction_count || 0; }
        else if (sort_by === "downloads") { aVal = a.downloads || 0; bVal = b.downloads || 0; }
        else if (sort_by === "rating") { aVal = a.avg_rating || 0; bVal = b.avg_rating || 0; }
        if (typeof aVal === "string" && typeof bVal === "string") {
          return sort_order === "asc" ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
        }
        return sort_order === "asc"
          ? (aVal as number) - (bVal as number)
          : (bVal as number) - (aVal as number);
      });
    }

    const total = results.length;
    const start = (page - 1) * page_size;
    const items = results.slice(start, start + page_size);
    return {
      items,
      total,
      page,
      page_size,
      total_pages: Math.ceil(total / page_size),
    };
  }

  const searchParams = new URLSearchParams();
  if (params.query) searchParams.set("query", params.query);
  if (params.network) searchParams.set("network", params.network);
  if (params.networks?.length) {
    params.networks.forEach((n) => searchParams.append("networks", n));
  }
  if (params.verified_only) searchParams.set("verified_only", "true");
  if (params.category) searchParams.set("category", params.category);
  if (params.categories?.length) {
    params.categories.forEach((c) => searchParams.append("categories", c));
  }
  if (params.tags?.length) params.tags.forEach((t) => searchParams.append("tags", t));
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  if (params.sort_by) searchParams.set("sort_by", params.sort_by);
  if (params.sort_order) searchParams.set("sort_order", params.sort_order);

  return apiFetch<PaginatedResponse<Contract>>(`/api/contracts?${searchParams.toString()}`);
}

export async function advancedSearchContracts(
  params: {
    query: QueryNode;
    limit?: number;
    offset?: number;
    sort_by?: ContractSearchParams["sort_by"];
    sort_order?: ContractSearchParams["sort_order"];
  },
): Promise<PaginatedResponse<Contract>> {
  return apiFetch<PaginatedResponse<Contract>>("/api/contracts/search", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

export async function fetchContract(id: string, network?: Network): Promise<ContractGetResponse> {
  if (USE_MOCKS) {
    const contract = MOCK_CONTRACTS.find(
      (c) => c.id === id || c.contract_id === id,
    ) as ContractGetResponse | undefined;
    if (!contract) throw new ApiError("Contract not found", 404);
    return { ...contract, current_network: network };
  }
  const qs = network ? `?network=${network}` : "";
  return apiFetch<ContractGetResponse>(`/api/contracts/${id}${qs}`);
}

export async function fetchContractHealth(id: string): Promise<ContractHealth> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      status: "healthy",
      last_activity: new Date().toISOString(),
      security_score: 85,
      total_score: 85,
      recommendations: [],
      updated_at: new Date().toISOString(),
    };
  }
  return apiFetch<ContractHealth>(`/api/contracts/${id}/health`);
}

export async function fetchContractAnalytics(id: string): Promise<ContractAnalyticsResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      deployments: { count: 0, unique_users: 0, by_network: {} },
      interactors: { unique_count: 0, top_users: [] },
      timeline: [],
    };
  }
  return apiFetch<ContractAnalyticsResponse>(`/api/contracts/${id}/analytics`);
}

export async function fetchContractVersions(id: string): Promise<ContractVersion[]> {
  if (USE_MOCKS) {
    return (MOCK_VERSIONS[id] || []) as ContractVersion[];
  }
  return apiFetch<ContractVersion[]>(`/api/contracts/${id}/versions`);
}

export async function fetchContractAbi(id: string, version?: string): Promise<ContractAbiResponse> {
  if (USE_MOCKS) {
    return { abi: null };
  }
  const qs = version ? `?version=${version}` : "";
  return apiFetch<ContractAbiResponse>(`/api/contracts/${id}/abi${qs}`);
}

export async function fetchContractChangelog(id: string): Promise<ContractChangelogResponse> {
  if (USE_MOCKS) {
    return { contract_id: id, entries: [] };
  }
  return apiFetch<ContractChangelogResponse>(`/api/contracts/${id}/changelog`);
}

export async function fetchContractRecommendations(
  id: string,
): Promise<ContractRecommendationsResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      algorithm: "mock",
      ab_variant: "a",
      cached: false,
      generated_at: new Date().toISOString(),
      recommendations: [],
    };
  }
  return apiFetch<ContractRecommendationsResponse>(`/api/contracts/${id}/recommendations`);
}

export async function fetchContractInteractions(
  id: string,
  queryParams: InteractionsQueryParams = {},
): Promise<InteractionsListResponse> {
  if (USE_MOCKS) {
    return { items: [], total: 0, limit: queryParams.limit || 20, offset: queryParams.offset || 0 };
  }
  const searchParams = new URLSearchParams();
  if (queryParams.limit) searchParams.set("limit", String(queryParams.limit));
  if (queryParams.offset) searchParams.set("offset", String(queryParams.offset));
  if (queryParams.account) searchParams.set("account", queryParams.account);
  if (queryParams.method) searchParams.set("method", queryParams.method);
  return apiFetch<InteractionsListResponse>(
    `/api/contracts/${id}/interactions?${searchParams.toString()}`,
  );
}

export async function publishContract(data: PublishRequest): Promise<Contract> {
  return apiFetch<Contract>("/api/contracts", {
    method: "POST",
    body: JSON.stringify(data),
  });
}
