/**
 * Registry-wide queries: the headline statistics and the template catalogue.
 *
 * `useRegistryStats` is the `getStats` endpoint — the one the home page and the
 * contracts list read. It is not `hooks/useStats.ts`, which polls the separate
 * period-scoped `fetchStats` endpoint for the analytics page.
 */

import { useQuery } from "@tanstack/react-query";
import { getStats, fetchStats } from "@/lib/api/stats";
import { fetchAnalytics } from "@/lib/api/analytics";
import { fetchTemplates } from "@/lib/api/templates";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type {
  AnalyticsResponse,
  LegacyStatsResponse,
  StatsResponse,
  Template,
  TimePeriod,
} from "@/types";

export function useRegistryStats(options?: QueryOpts<LegacyStatsResponse>) {
  return useQuery({
    queryKey: queryKeys.stats(),
    queryFn: () => getStats(),
    ...options,
  });
}

/**
 * The period-scoped statistics the analytics page shows, refreshed every 30
 * seconds while the tab is visible. A different endpoint from
 * `useRegistryStats` and a different response shape, which is why it is a
 * separate hook rather than an argument.
 */
export function useStatsForPeriod(
  period: TimePeriod,
  options?: QueryOpts<StatsResponse>,
) {
  return useQuery({
    queryKey: queryKeys.statsForPeriod(period),
    queryFn: () => fetchStats(period),
    refetchInterval: 30_000,
    refetchIntervalInBackground: false,
    ...options,
  });
}

/**
 * Registry analytics for a period, refreshed every minute while visible.
 *
 * Not `hooks/useAnalytics.ts`, which is the event-logging hook every page uses
 * — hence the longer name.
 */
export function useRegistryAnalytics(
  period: TimePeriod,
  options?: QueryOpts<AnalyticsResponse>,
) {
  return useQuery({
    queryKey: queryKeys.analytics(period),
    queryFn: () => fetchAnalytics(period),
    refetchInterval: 60_000,
    refetchIntervalInBackground: false,
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
