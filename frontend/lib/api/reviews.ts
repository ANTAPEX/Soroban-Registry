/**
 * Collaborative review endpoints.
 */

import { apiFetch } from "./client";
import type {
  CollaborativeComment,
  CollaborativeReview,
  CollaborativeReviewDetails,
  CreateCollaborativeReviewRequest,
} from "@/types";

export async function fetchCollaborativeReview(
  contractId: string,
): Promise<CollaborativeReviewDetails> {
  return apiFetch<CollaborativeReviewDetails>(`/api/contracts/${contractId}/review`);
}

export async function createCollaborativeReview(
  request: CreateCollaborativeReviewRequest,
): Promise<CollaborativeReview> {
  return apiFetch<CollaborativeReview>("/api/reviews/collaborative", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export async function addReviewComment(
  contractId: string,
  comment: Partial<CollaborativeComment>,
): Promise<CollaborativeComment> {
  return apiFetch<CollaborativeComment>(`/api/contracts/${contractId}/review/comments`, {
    method: "POST",
    body: JSON.stringify(comment),
  });
}

export async function updateReviewerStatus(
  reviewId: string,
  status: string,
): Promise<void> {
  return apiFetch<void>(`/api/reviews/collaborative/${reviewId}/status`, {
    method: "PATCH",
    body: JSON.stringify({ status }),
  });
}
