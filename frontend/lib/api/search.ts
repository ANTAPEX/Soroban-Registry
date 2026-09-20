/**
 * Search endpoints: suggestions, semantic search and saved searches.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import { fetchContracts } from "./contracts";
import { detectIntent } from "./semanticRanking";
import type {
  ContractSearchParams,
  FavoriteSearch,
  SearchSuggestionsResponse,
  SemanticContractSearchResponse,
} from "@/types";

export async function fetchSearchSuggestions(query: string): Promise<SearchSuggestionsResponse> {
  if (USE_MOCKS || !query.trim()) {
    return { items: [] };
  }
  return apiFetch<SearchSuggestionsResponse>(
    `/api/search/suggestions?query=${encodeURIComponent(query)}`,
  );
}

export async function semanticSearch(
  params: ContractSearchParams,
): Promise<SemanticContractSearchResponse> {
  if (USE_MOCKS) {
    const base = await fetchContracts(params);
    const intent = detectIntent(params.query || "", params);
    return {
      ...base,
      semantic: {
        raw_query: params.query || "",
        interpreted_query: params.query || "",
        intent,
        fallback_used: false,
        query_suggestions: [],
      },
    };
  }
  const searchParams = new URLSearchParams();
  if (params.query) searchParams.set("query", params.query);
  if (params.network) searchParams.set("network", params.network);
  if (params.verified_only) searchParams.set("verified_only", "true");
  if (params.category) searchParams.set("category", params.category);
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  return apiFetch<SemanticContractSearchResponse>(
    `/api/search/semantic?${searchParams.toString()}`,
  );
}

export async function listFavoriteSearches(): Promise<FavoriteSearch[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<FavoriteSearch[]>("/api/favorites/search");
}

export async function deleteFavoriteSearch(id: string): Promise<void> {
  await apiFetch<void>(`/api/favorites/search/${id}`, {
    method: "DELETE",
  });
}

export async function fetchContractSearchSuggestions(
  query: string,
  limit?: number,
): Promise<SearchSuggestionsResponse> {
  if (USE_MOCKS || !query.trim()) {
    return { items: [] };
  }
  const search = new URLSearchParams();
  search.set("query", query);
  if (limit != null) search.set("limit", String(limit));
  return apiFetch<SearchSuggestionsResponse>(`/api/search/suggestions?${search.toString()}`);
}
