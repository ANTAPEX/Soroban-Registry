/**
 * Custom metric queries, over `lib/api/metrics.ts`.
 */

import { useQuery } from "@tanstack/react-query";
import {
  fetchCustomMetricCatalog,
  fetchCustomMetricSeries,
} from "@/lib/api/metrics";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type { MetricCatalogEntry, MetricSeriesResponse } from "@/types";

export function useCustomMetricsCatalog(
  contractId: string,
  options?: QueryOpts<MetricCatalogEntry[]>,
) {
  return useQuery({
    queryKey: queryKeys.customMetricsCatalog(contractId),
    queryFn: () => fetchCustomMetricCatalog(contractId),
    ...options,
  });
}

export function useCustomMetricsSeries(
  contractId: string,
  metricName: string,
  resolution: "hour" | "day" | "raw",
  limit = 48,
  options?: QueryOpts<MetricSeriesResponse>,
) {
  return useQuery({
    queryKey: queryKeys.customMetricsSeries(contractId, metricName, resolution),
    queryFn: () =>
      fetchCustomMetricSeries(contractId, metricName || "", {
        resolution,
        limit,
      }),
    enabled: !!metricName,
    ...options,
  });
}
