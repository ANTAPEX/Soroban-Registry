/**
 * Activity feed endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type {
  ActivityFeedParams,
  ActivityFeedResponse,
  AnalyticsEvent,
} from "@/types";

export async function fetchActivityFeed(
  params: ActivityFeedParams = {},
): Promise<ActivityFeedResponse> {
  if (USE_MOCKS) {
    return { items: [], total: 0, limit: params.limit || 20, next_cursor: null };
  }
  const searchParams = new URLSearchParams();
  if (params.cursor) searchParams.set("cursor", params.cursor);
  if (params.limit) searchParams.set("limit", String(params.limit));
  if (params.event_type) searchParams.set("event_type", params.event_type);
  if (params.contract_id) searchParams.set("contract_id", params.contract_id);
  // Backend returns a `CursorPaginatedResponse<AnalyticsEvent>` shaped as
  // { data, total, has_more, next_cursor } — adapt it to the `items`/`limit`
  // shape the rest of the frontend expects.
  const raw = await apiFetch<{
    data: AnalyticsEvent[];
    total: number;
    has_more: boolean;
    next_cursor: string | null;
  }>(`/api/activity-feed?${searchParams.toString()}`);
  return {
    items: raw.data,
    total: raw.total,
    limit: params.limit || 20,
    next_cursor: raw.next_cursor,
  };
}
