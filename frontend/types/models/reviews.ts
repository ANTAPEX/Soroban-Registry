/**
 * Review and collaboration types
 */

export interface CollaborativeReview {
  id: string;
  contract_id: string;
  version: string;
  status: "pending" | "approved" | "changes_requested";
  created_at: string;
  updated_at: string;
}

export interface CollaborativeReviewer {
  id: string;
  review_id: string;
  user_id: string;
  status: "pending" | "approved" | "changes_requested";
  created_at: string;
  updated_at: string;
}

export interface CollaborativeComment {
  id: string;
  review_id: string;
  user_id: string;
  content: string;
  line_number?: number;
  file_path?: string;
  abi_path?: string;
  parent_id?: string;
  created_at: string;
  updated_at: string;
}

export interface CollaborativeReviewDetails {
  review: CollaborativeReview;
  reviewers: CollaborativeReviewer[];
  comments: CollaborativeComment[];
}

export interface Comment {
  id: string;
  author: string;
  body: string;
  created_at: string;
  flagged: boolean;
  score: number;
  flag_count: number;
  parent_id?: string | null;
  line_number?: number | null;
  file_path?: string | null;
  abi_path?: string | null;
}

export interface CommentListResponse {
  items: Comment[];
  total: number;
}

export interface CreateCollaborativeReviewRequest {
  contract_id: string;
  version: string;
  reviewer_ids: string[];
}
