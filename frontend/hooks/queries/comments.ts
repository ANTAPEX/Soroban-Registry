/**
 * Contract comment queries and mutations, over `lib/api/comments.ts`.
 *
 * Each mutation invalidates the comment list itself, so a call site no longer
 * has to remember to.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchComments,
  postComment,
  voteComment,
  flagComment,
} from "@/lib/api/comments";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type { Comment, CommentListResponse } from "@/types";

export function useContractComments(
  contractId: string,
  options?: QueryOpts<CommentListResponse>,
) {
  return useQuery({
    queryKey: queryKeys.contractComments(contractId),
    queryFn: () => fetchComments(contractId),
    ...options,
  });
}

function useCommentMutation<TVariables>(
  contractId: string,
  mutationFn: (variables: TVariables) => Promise<Comment>,
  options?: MutationOpts<Comment, TVariables>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn,
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.contractComments(contractId),
      });
      options?.onSuccess?.(...args);
    },
  });
}

export function usePostComment(
  contractId: string,
  parentId?: string,
  options?: MutationOpts<Comment, string>,
) {
  return useCommentMutation<string>(
    contractId,
    (body) => postComment(contractId, body, parentId),
    options,
  );
}

export function useVoteComment(
  commentId: string,
  contractId: string,
  options?: MutationOpts<Comment, "up" | "down">,
) {
  return useCommentMutation<"up" | "down">(
    contractId,
    (direction) => voteComment(commentId, contractId, direction),
    options,
  );
}

export function useFlagComment(
  commentId: string,
  contractId: string,
  reason = "spam",
  options?: MutationOpts<Comment, void>,
) {
  return useCommentMutation<void>(
    contractId,
    () => flagComment(commentId, contractId, reason),
    options,
  );
}
