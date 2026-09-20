/**
 * Contract recommendation types for Soroban Registry
 */

import type { Network } from "./network";
export interface RecommendationReason {
  code: string;
  message: string;
  weight: number;
}

export interface RecommendedContract {
  id: string;
  contract_id: string;
  name: string;
  description?: string;
  network: Network;
  category?: string;
  popularity_score: number;
  similarity_score: number;
  recommendation_score: number;
  reasons: RecommendationReason[];
  explanation: string;
}

export interface ContractRecommendationsResponse {
  contract_id: string;
  algorithm: string;
  ab_variant: string;
  cached: boolean;
  generated_at: string;
  recommendations: RecommendedContract[];
}
