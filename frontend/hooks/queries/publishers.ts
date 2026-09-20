/**
 * Publisher queries, over `lib/api/publishers.ts`.
 */

import { useQuery } from "@tanstack/react-query";
import { getPublisher } from "@/lib/api/publishers";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type { PublisherResponse } from "@/types";

export function usePublisher(
  address: string | undefined,
  options?: QueryOpts<PublisherResponse>,
) {
  return useQuery({
    queryKey: queryKeys.publisher(address),
    queryFn: () => getPublisher(address!),
    enabled: !!address,
    ...options,
  });
}
