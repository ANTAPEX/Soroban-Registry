import { Network } from "./network";
import { Contract } from "./contract";
import { PaginatedResponse } from "./common";

/**
 * Search related types
 */

export interface SearchSuggestion {
  text: string;
  kind: string;
  score: number;
}

export interface SearchSuggestionsResponse {
  items: SearchSuggestion[];
}

export type SearchIntentType =
  | "generic"
  | "category"
  | "network"
  | "verification"
  | "tag"
  | "author";

export interface SearchIntent {
  type: SearchIntentType;
  confidence: number;
  extracted: {
    categories: string[];
    tags: string[];
    networks: Network[];
    verified_only: boolean;
    author?: string;
  };
}

export interface SemanticSearchMetadata {
  raw_query: string;
  interpreted_query: string;
  intent: SearchIntent;
  fallback_used: boolean;
  query_suggestions: string[];
}

export interface SemanticContractSearchResponse extends PaginatedResponse<Contract> {
  semantic: SemanticSearchMetadata;
}
