"use client";

import { useCallback } from "react";
import { useRegistryAnalytics } from "@/hooks/queries";
import type { AnalyticsResponse, TimePeriod } from "@/types";

interface UseAnalyticsDataReturn {
  data: AnalyticsResponse | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

/**
 * Registry analytics, keeping the `{ data, loading, error, refetch }` shape its
 * callers already use. See `useStats` for the two differences that come with
 * moving the hand-rolled polling onto React Query.
 */
export function useAnalyticsData(period: TimePeriod): UseAnalyticsDataReturn {
  const query = useRegistryAnalytics(period);

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
