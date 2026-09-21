/**
 * Formal verification queries, over `lib/api/verification.ts`.
 */

import { useQuery } from "@tanstack/react-query";
import { fetchFormalVerificationResults } from "@/lib/api/verification";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts } from "./types";
import type { FormalVerificationReport } from "@/types";

export function useFormalVerification(
  contractId: string,
  options?: QueryOpts<FormalVerificationReport[]>,
) {
  return useQuery({
    queryKey: queryKeys.formalVerification(contractId),
    queryFn: () => fetchFormalVerificationResults(contractId),
    ...options,
  });
}
