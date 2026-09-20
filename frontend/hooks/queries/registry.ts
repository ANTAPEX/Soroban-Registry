/**
 * Registry-wide queries: the headline statistics and the template catalogue.
 *
 * `useRegistryStats` is the `getStats` endpoint — the one the home page and the
 * contracts list read. It is not `hooks/useStats.ts`, which polls the separate
 * period-scoped `fetchStats` endpoint for the analytics page.
 */

import { useQuery } from "@tanstack/react-query";
import { getStats } from "@/lib/api/stats";
import { fetchTemplates } from "@/lib/api/templates";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type { LegacyStatsResponse, Template } from "@/types";

export function useRegistryStats(options?: QueryOpts<LegacyStatsResponse>) {
  return useQuery({
    queryKey: queryKeys.stats(),
    queryFn: () => getStats(),
    ...options,
  });
}

export function useTemplates(options?: QueryOpts<Template[]>) {
  return useQuery({
    queryKey: queryKeys.templates(),
    queryFn: () => fetchTemplates(),
    ...options,
  });
}
