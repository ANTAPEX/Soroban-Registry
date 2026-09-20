/**
 * Release notes queries and mutations, over `lib/api/releaseNotes.ts`.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listReleaseNotes,
  generateReleaseNotes,
  updateReleaseNotes,
  publishReleaseNotes,
} from "@/lib/api/releaseNotes";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type { ReleaseNotesResponse } from "@/types";

export function useReleaseNotes(
  contractId: string,
  options?: QueryOpts<ReleaseNotesResponse[]>,
) {
  return useQuery({
    queryKey: queryKeys.releaseNotes(contractId),
    queryFn: () => listReleaseNotes(contractId),
    ...options,
  });
}

function useReleaseNotesMutation<TVariables>(
  contractId: string,
  mutationFn: (variables: TVariables) => Promise<ReleaseNotesResponse>,
  options?: MutationOpts<ReleaseNotesResponse, TVariables>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn,
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.releaseNotes(contractId),
      });
      options?.onSuccess?.(...args);
    },
  });
}

export function useGenerateReleaseNotes(
  contractId: string,
  options?: MutationOpts<ReleaseNotesResponse, string>,
) {
  return useReleaseNotesMutation<string>(
    contractId,
    (version) => generateReleaseNotes(contractId, { version }),
    options,
  );
}

export function useUpdateReleaseNotes(
  contractId: string,
  options?: MutationOpts<
    ReleaseNotesResponse,
    { version: string; text: string }
  >,
) {
  return useReleaseNotesMutation<{ version: string; text: string }>(
    contractId,
    ({ version, text }) =>
      updateReleaseNotes(contractId, version, { notes_text: text }),
    options,
  );
}

export function usePublishReleaseNotes(
  contractId: string,
  options?: MutationOpts<ReleaseNotesResponse, string>,
) {
  return useReleaseNotesMutation<string>(
    contractId,
    (version) => publishReleaseNotes(contractId, version),
    options,
  );
}
