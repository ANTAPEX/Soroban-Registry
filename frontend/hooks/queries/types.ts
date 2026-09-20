/**
 * Shared option types for the query hooks.
 *
 * Every hook here takes the arguments its endpoint needs and forwards an
 * `options` object to React Query, so a call site can still set `enabled`,
 * `staleTime`, `retry` or `placeholderData` without reaching for `useQuery`
 * directly. `queryKey` and `queryFn` are deliberately not overridable: owning
 * those is the whole point of the layer.
 */

import type { UseQueryOptions, UseMutationOptions } from "@tanstack/react-query";

export type QueryOpts<TData> = Omit<
  UseQueryOptions<TData, Error, TData>,
  "queryKey" | "queryFn"
>;

export type MutationOpts<TData, TVariables> = Omit<
  UseMutationOptions<TData, Error, TVariables>,
  "mutationFn"
>;
