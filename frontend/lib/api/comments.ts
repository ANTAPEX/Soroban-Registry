/**
 * Contract comment endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { Comment, CommentListResponse } from "@/types";

export async function fetchComments(contractId: string): Promise<CommentListResponse> {
  if (USE_MOCKS) {
    return { items: [], total: 0 };
  }
  return apiFetch<CommentListResponse>(`/api/contracts/${contractId}/comments`);
}

export async function postComment(
  contractId: string,
  body: string,
  parentId?: string,
): Promise<Comment> {
  return apiFetch<Comment>(`/api/contracts/${contractId}/comments`, {
    method: "POST",
    body: JSON.stringify({ body, parent_id: parentId }),
  });
}

export async function voteComment(
  commentId: string,
  contractId: string,
  direction: "up" | "down",
): Promise<Comment> {
  return apiFetch<Comment>(`/api/contracts/${contractId}/comments/${commentId}/vote`, {
    method: "POST",
    body: JSON.stringify({ direction }),
  });
}

export async function flagComment(
  commentId: string,
  contractId: string,
  reason: string,
): Promise<Comment> {
  return apiFetch<Comment>(`/api/contracts/${contractId}/comments/${commentId}/flag`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}
