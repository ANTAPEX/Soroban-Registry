"use client";

import { useCallback } from "react";
import { useStatsForPeriod } from "@/hooks/queries";
import type { StatsResponse, TimePeriod } from "@/types";

interface UseStatsReturn {
  data: StatsResponse | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

/**
 * The analytics page's statistics, with its existing `{ data, loading, error,
 * refetch }` shape kept so the page did not have to change.
 *
 * The polling this used to hand-roll — a 30 second interval that skipped hidden
 * tabs — is now React Query's `refetchInterval` with
 * `refetchIntervalInBackground: false`, which does the same thing and also
 * dedupes and caches across mounts.
 *
 * One deliberate difference: `loading` is now true only while there is nothing
 * to show. Pressing refresh keeps the current numbers on screen until the new
 * ones arrive, where before the whole page fell back to its skeleton.
 */
export function useStats(period: TimePeriod): UseStatsReturn {
  const query = useStatsForPeriod(period);

  const refetch = useCallback(async () => {
    await query.refetch();
  }, [query]);

  return {
    data: query.data ?? null,
    loading: query.isPending,
    error: query.error ?? null,
    refetch,
  };
}
