/**
 * Collaborative review endpoints.
 *
 * Every one of these is keyed by the *review* id, not the contract id. The
 * routes are defined in `collaborative_review_routes()` in the backend's
 * `routes.rs`; `/api/contracts/:id/reviews` is a different feature (public
 * star ratings) and has nothing to do with these.
 */

import { apiFetch } from "./client";
import type {
  CollaborativeComment,
  CollaborativeReview,
  CollaborativeReviewDetails,
  CreateCollaborativeReviewRequest,
} from "@/types";

export async function fetchCollaborativeReview(
  reviewId: string,
): Promise<CollaborativeReviewDetails> {
  return apiFetch<CollaborativeReviewDetails>(
    `/api/reviews/collaborative/${reviewId}`,
  );
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
  reviewId: string,
  comment: Partial<CollaborativeComment>,
): Promise<CollaborativeComment> {
  return apiFetch<CollaborativeComment>(
    `/api/reviews/collaborative/${reviewId}/comment`,
    {
      method: "POST",
      body: JSON.stringify(comment),
    },
  );
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
