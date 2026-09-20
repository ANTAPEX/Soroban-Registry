/**
 * Activity feed queries, over `lib/api/activity.ts`.
 *
 * The contract timeline is the same endpoint scoped to one contract, under its
 * own cache key so a contract page and the global feed do not evict each other.
 */

import { useQuery } from "@tanstack/react-query";
import { fetchActivityFeed } from "@/lib/api/activity";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type { ActivityFeedResponse, AnalyticsEventType } from "@/types";

/**
 * `eventType` accepts the UI's "all" sentinel as well as a real event type, so
 * the filter control can pass its state straight through. "all" stays in the
 * cache key — it is the key the feed has always used for the unfiltered view.
 */
export function useActivityFeed(
  eventType: AnalyticsEventType | "all" = "all",
  limit = 20,
  options?: QueryOpts<ActivityFeedResponse>,
) {
  return useQuery({
    queryKey: queryKeys.activityFeed(eventType),
    queryFn: () =>
      fetchActivityFeed({
        event_type: eventType === "all" ? undefined : eventType,
        limit,
      }),
    ...options,
  });
}

export function useContractTimeline(
  contractId: string,
  limit = 100,
  options?: QueryOpts<ActivityFeedResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractTimeline(contractId),
    queryFn: () => fetchActivityFeed({ contract_id: contractId, limit }),
    ...options,
  });
}
