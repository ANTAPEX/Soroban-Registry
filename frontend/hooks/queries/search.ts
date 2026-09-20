/**
 * Saved search queries, over `lib/api/search.ts`.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listFavoriteSearches,
  deleteFavoriteSearch,
  fetchContractSearchSuggestions,
} from "@/lib/api/search";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type { FavoriteSearch, SearchSuggestionsResponse } from "@/types";

export function useFavoriteSearches(options?: QueryOpts<FavoriteSearch[]>) {
  return useQuery({
    queryKey: queryKeys.favoriteSearches(),
    queryFn: () => listFavoriteSearches(),
    ...options,
  });
}

export function useDeleteFavoriteSearch(
  options?: MutationOpts<void, string>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteFavoriteSearch(id),
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.favoriteSearches(),
      });
      options?.onSuccess?.(...args);
    },
  });
}

/**
 * Type-ahead suggestions for a search box.
 *
 * Debounce the query before passing it in — this keys the cache on whatever it
 * is given, so an undebounced value would cache a request per keystroke.
 */
export function useContractSearchSuggestions(
  query: string,
  limit = 8,
  options?: QueryOpts<SearchSuggestionsResponse>,
) {
  return useQuery({
    queryKey: queryKeys.searchSuggestions(query, limit),
    queryFn: () => fetchContractSearchSuggestions(query, limit),
    enabled: query.trim().length > 0,
    ...options,
  });
}
