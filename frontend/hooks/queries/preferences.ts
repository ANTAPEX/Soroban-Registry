/**
 * User preference queries and mutations, over `lib/api/preferences.ts`.
 *
 * Both take the auth token explicitly rather than reading it themselves: the
 * token lives in localStorage and can change under a mounted component, and a
 * query keyed on a value it reads for itself would not refetch when it did.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { fetchPreferences, updatePreferences } from "@/lib/api/preferences";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type { UserPreferences } from "@/types";

export function usePreferences(
  token: string | null,
  options?: QueryOpts<UserPreferences>,
) {
  return useQuery({
    queryKey: queryKeys.preferences(),
    queryFn: () => fetchPreferences(token!),
    enabled: !!token,
    ...options,
  });
}

export function useUpdatePreferences(
  token: string | null,
  options?: MutationOpts<UserPreferences, string[]>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (favorites: string[]) => updatePreferences(token!, favorites),
    // One retry after three seconds, which is what the favorites hook used to
    // do with its own setTimeout.
    retry: 1,
    retryDelay: 3000,
    ...options,
    onSuccess: (data, ...rest) => {
      queryClient.setQueryData(queryKeys.preferences(), data);
      options?.onSuccess?.(data, ...rest);
    },
  });
}
