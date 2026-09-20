/**
 * Custom metric catalogue and series endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { MetricCatalogEntry, MetricSeriesResponse } from "@/types";

export async function fetchMetricCatalog(contractId: string): Promise<MetricCatalogEntry[]> {
  if (USE_MOCKS) return [];
  return apiFetch<MetricCatalogEntry[]>(`/api/contracts/${contractId}/metrics`);
}

export async function fetchMetricSeries(
  contractId: string,
  metricName: string,
  params: { from?: string; to?: string; resolution?: "hour" | "day" | "raw" } = {},
): Promise<MetricSeriesResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      metric_name: metricName,
      metric_type: null,
      resolution: params.resolution || "day",
      points: [],
    };
  }
  const searchParams = new URLSearchParams();
  if (params.from) searchParams.set("from", params.from);
  if (params.to) searchParams.set("to", params.to);
  if (params.resolution) searchParams.set("resolution", params.resolution);
  return apiFetch<MetricSeriesResponse>(
    `/api/contracts/${contractId}/metrics/${encodeURIComponent(metricName)}?${searchParams.toString()}`,
  );
}

export async function fetchCustomMetricCatalog(contractId: string): Promise<MetricCatalogEntry[]> {
  if (USE_MOCKS) return [];
  return apiFetch<MetricCatalogEntry[]>(`/api/contracts/${contractId}/metrics/catalog`);
}

export async function fetchCustomMetricSeries(
  contractId: string,
  metricName: string,
  params: { from?: string; to?: string; resolution?: "hour" | "day" | "raw"; limit?: number } = {},
): Promise<MetricSeriesResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      metric_name: metricName,
      metric_type: null,
      resolution: params.resolution || "day",
      points: [],
    };
  }
  const searchParams = new URLSearchParams();
  if (params.from) searchParams.set("from", params.from);
  if (params.to) searchParams.set("to", params.to);
  if (params.resolution) searchParams.set("resolution", params.resolution);
  if (params.limit) searchParams.set("limit", String(params.limit));
  return apiFetch<MetricSeriesResponse>(
    `/api/contracts/${contractId}/metrics/${encodeURIComponent(metricName)}?${searchParams.toString()}`,
  );
}
