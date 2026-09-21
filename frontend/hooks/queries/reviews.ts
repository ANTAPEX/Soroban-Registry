/**
 * Collaborative review queries and mutations, over `lib/api/reviews.ts`.
 *
 * Every id here is a *review* id. A contract id will 404: the collaborative
 * review routes hang off `/api/reviews/collaborative`, not off the contract.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchCollaborativeReview,
  createCollaborativeReview,
  addReviewComment,
  updateReviewerStatus,
} from "@/lib/api/reviews";
import { queryKeys } from "@/lib/queryKeys";
import type { QueryOpts, MutationOpts } from "./types";
import type {
  CollaborativeComment,
  CollaborativeReview,
  CollaborativeReviewDetails,
  CreateCollaborativeReviewRequest,
} from "@/types";

export function useCollaborativeReview(
  id: string | null | undefined,
  options?: QueryOpts<CollaborativeReviewDetails>,
) {
  return useQuery({
    queryKey: queryKeys.collaborativeReview(id),
    queryFn: () => fetchCollaborativeReview(id!),
    enabled: !!id,
    ...options,
  });
}

type NewReviewComment = Pick<
  CollaborativeComment,
  "content" | "line_number" | "abi_path"
>;

export function useAddReviewComment(
  id: string | null | undefined,
  options?: MutationOpts<CollaborativeComment, NewReviewComment>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: NewReviewComment) => addReviewComment(id!, data),
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.collaborativeReview(id),
      });
      options?.onSuccess?.(...args);
    },
  });
}

export function useUpdateReviewerStatus(
  id: string | null | undefined,
  options?: MutationOpts<void, string>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (status: string) => updateReviewerStatus(id!, status),
    ...options,
    onSuccess: (...args) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.collaborativeReview(id),
      });
      options?.onSuccess?.(...args);
    },
  });
}

export function useCreateCollaborativeReview(
  options?: MutationOpts<CollaborativeReview, CreateCollaborativeReviewRequest>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: CreateCollaborativeReviewRequest) =>
      createCollaborativeReview(request),
    ...options,
    onSuccess: (review, ...rest) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.collaborativeReview(review.id),
      });
      options?.onSuccess?.(review, ...rest);
    },
  });
}
