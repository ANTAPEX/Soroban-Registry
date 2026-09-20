/**
 * Saved search queries, over `lib/api/search.ts`.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listFavoriteSearches,
  deleteFavoriteSearch,
} from "@/lib/api/search";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type { FavoriteSearch } from "@/types";

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
