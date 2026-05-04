// Mock data: conditionally imported only in development/test.
// In production (NEXT_PUBLIC_USE_MOCKS !== "true"), these are empty stubs
// that never get reached (gated behind USE_MOCKS checks below).
/* eslint-disable @typescript-eslint/no-explicit-any */
let MOCK_CONTRACTS: any[] = [];
let MOCK_EXAMPLES: Record<string, any[]> = {};
let MOCK_VERSIONS: Record<string, any[]> = {};
/* eslint-enable @typescript-eslint/no-explicit-any */
if (process.env.NEXT_PUBLIC_USE_MOCKS === "true") {
  // Dynamic require ensures Next.js tree-shakes mock-data from production bundles
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const mocks = require("./mock-data");
  MOCK_CONTRACTS = mocks.MOCK_CONTRACTS;
  MOCK_EXAMPLES = mocks.MOCK_EXAMPLES;
  MOCK_VERSIONS = mocks.MOCK_VERSIONS;
}
import { CollaborativeComment, CollaborativeReviewDetails, VerificationLevel } from "@/types";
import { trackEvent } from "./analytics";
import {
  ApiError,
  NetworkError,
  extractErrorData,
  createApiError,
} from "./errors";

export type Network = "mainnet" | "testnet" | "futurenet";

export type NetworkStatus = "online" | "offline" | "degraded";

export interface NetworkEndpoints {
  rpc_url: string;
  health_url: string;
  explorer_url: string;
  friendbot_url?: string;
}

export interface NetworkInfo {
  id: string;
  name: string;
  network_type: Network;
  status: NetworkStatus;
  endpoints: NetworkEndpoints;
  last_checked_at: string;
  last_indexed_ledger_height?: number;
  last_indexed_at?: string;
  consecutive_failures: number;
  status_message?: string;
}

export interface NetworkListResponse {
  networks: NetworkInfo[];
  cached_at: string;
}

/** Per-network config (Issue #43) */
export interface NetworkConfig {
  contract_id: string;
  is_verified: boolean;
  min_version?: string;
  max_version?: string;
}

export interface Contract {
  id: string;
  contract_id: string;
  wasm_hash: string;
  name: string;
  description?: string;
  publisher_id: string;
  network: Network;
  is_verified: boolean;
  verification_level?: VerificationLevel;
  category?: string;
  tags: string[];
  popularity_score?: number;
  downloads?: number;
  average_rating?: number;
  avg_rating?: number;
  review_count?: number;
  deployment_count?: number;
  interaction_count?: number;
  relevance_score?: number;
  // Image fields for contract logo/icon
  logo_url?: string;
  created_at: string;
  updated_at: string;
  verified_at?: string;
  last_accessed_at?: string;
  is_maintenance?: boolean;
  /** Logical contract grouping (Issue #43) */
  logical_id?: string;
  /** Per-network configs: { mainnet: {...}, testnet: {...} } */
  network_configs?: Record<Network, NetworkConfig>;
}

/** GET /contracts/:id response when ?network= is used (Issue #43) */
export interface ContractGetResponse extends Contract {
  current_network?: Network;
  network_config?: NetworkConfig;
}

export interface ContractHealth {
  contract_id: string;
  status: "healthy" | "warning" | "critical";
  last_activity: string;
  security_score: number;
  audit_date?: string;
  total_score: number;
  recommendations: string[];
  updated_at: string;
}

export interface ContractInteractionResponse {
  id: string;
  account: string | null;
  method: string | null;
  parameters: unknown;
  return_value: unknown;
  transaction_hash: string | null;
  created_at: string;
}

export interface InteractionsQueryParams {
  limit?: number;
  offset?: number;
  account?: string;
  method?: string;
  from_timestamp?: string;
  to_timestamp?: string;
}

export interface InteractionsListResponse {
  items: ContractInteractionResponse[];
  total: number;
  limit: number;
  offset: number;
}

/** Analytics timeline entry (one day) */
export interface TimelineEntry {
  date: string;
  count: number;
}

export interface TopUser {
  address: string;
  count: number;
}

export interface InteractorStats {
  unique_count: number;
  top_users: TopUser[];
}

export interface DeploymentStats {
  count: number;
  unique_users: number;
  by_network: Record<string, number>;
}

export interface ContractAnalyticsResponse {
  contract_id: string;
  deployments: DeploymentStats;
  interactors: InteractorStats;
  timeline: TimelineEntry[];
}

export interface ContractVersion {
  id: string;
  contract_id: string;
  version: string;
  wasm_hash: string;
  source_url?: string;
  commit_hash?: string;
  release_notes?: string;
  created_at: string;
}

export interface ContractAbiResponse {
  abi: unknown;
}

export interface ContractChangelogEntry {
  version: string;
  created_at: string;
  commit_hash?: string;
  source_url?: string;
  release_notes?: string;
  breaking: boolean;
  breaking_changes: string[];
}

export interface ContractChangelogResponse {
  contract_id: string;
  entries: ContractChangelogEntry[];
}

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

export interface Publisher {
  id: string;
  stellar_address: string;
  username?: string;
  email?: string;
  github_url?: string;
  website?: string;
  // Image fields for publisher avatar
  avatar_url?: string;
  created_at: string;
}

export type AnalyticsEventType =
  | "contract_published"
  | "contract_verified"
  | "contract_deployed"
  | "version_created"
  | "contract_updated"
  | "publisher_created"
  | "search_click";

export interface AnalyticsEvent {
  id: string;
  event_type: AnalyticsEventType;
  contract_id: string;
  user_address: string | null;
  network: Network | null;
  metadata: Record<string, unknown> | null;
  created_at: string;
}

export interface ActivityFeedParams {
  cursor?: string;
  limit?: number;
  event_type?: AnalyticsEventType;
  contract_id?: string;
}

export interface ActivityFeedResponse {
  items: AnalyticsEvent[];
  total: number;
  limit: number;
  next_cursor: string | null;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface DependencyTreeNode {
  contract_id: string;
  name: string;
  current_version: string;
  constraint_to_parent: string;
  dependencies: DependencyTreeNode[];
}

export interface MaintenanceWindow {
  message: string;
  scheduled_end_at?: string;
}

export type MaturityLevel = "alpha" | "beta" | "stable" | "mature" | "legacy";

export interface ContractSearchParams {
  query?: string;
  contract_id?: string;
  network?: "mainnet" | "testnet" | "futurenet";
  networks?: Array<"mainnet" | "testnet" | "futurenet">;
  verified_only?: boolean;
  favorites_only?: boolean;
  favorites_list?: string[];
  category?: string;
  categories?: string[];
  language?: string;
  languages?: string[];
  author?: string;
  tags?: string[];
  maturity?: "alpha" | "beta" | "stable" | "mature" | "legacy";
  page?: number;
  page_size?: number;
  sort_by?:
    | "name"
    | "created_at"
    | "updated_at"
    | "popularity"
    | "deployments"
    | "interactions"
    | "relevance"
    | "downloads"
    | "rating";
  sort_order?: "asc" | "desc";
  date_from?: string;
  date_to?: string;
}

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

export interface SemanticContractSearchResponse
  extends PaginatedResponse<Contract> {
  semantic: SemanticSearchMetadata;
}

export interface PublishRequest {
  contract_id: string;
  name: string;
  description?: string;
  network: "mainnet" | "testnet" | "futurenet";
  category?: string;
  tags: string[];
  source_url?: string;
  publisher_address: string;
}

export type CustomMetricType = "counter" | "gauge" | "histogram";

export interface MetricCatalogEntry {
  metric_name: string;
  metric_type: CustomMetricType;
  last_seen: string;
  sample_count: number;
}

export interface MetricSeriesPoint {
  bucket_start: string;
  bucket_end: string;
  sample_count: number;
  sum_value?: number;
  avg_value?: number;
  min_value?: number;
  max_value?: number;
  p50_value?: number;
  p95_value?: number;
  p99_value?: number;
}

export interface MetricSample {
  timestamp: string;
  value: number;
  unit?: string;
  metadata?: Record<string, unknown> | null;
}

export interface MetricSeriesResponse {
  contract_id: string;
  metric_name: string;
  metric_type: CustomMetricType | null;
  resolution: "hour" | "day" | "raw";
  points?: MetricSeriesPoint[];
  samples?: MetricSample[];
}

export type DeprecationStatus = "active" | "deprecated" | "retired";

export type ReleaseNotesStatus = "draft" | "published";

export interface FunctionChange {
  name: string;
  change_type: "added" | "removed" | "modified";
  old_signature?: string;
  new_signature?: string;
  is_breaking: boolean;
}

export interface DiffSummary {
  files_changed: number;
  lines_added: number;
  lines_removed: number;
  function_changes: FunctionChange[];
  has_breaking_changes: boolean;
  features_count: number;
  fixes_count: number;
  breaking_count: number;
}

export interface ReleaseNotesResponse {
  id: string;
  contract_id: string;
  version: string;
  previous_version?: string;
  diff_summary: DiffSummary;
  changelog_entry?: string;
  notes_text: string;
  status: ReleaseNotesStatus;
  generated_by: string;
  created_at: string;
  updated_at: string;
  published_at?: string;
}

export interface GenerateReleaseNotesRequest {
  version: string;
  previous_version?: string;
  source_url?: string;
  changelog_content?: string;
  contract_address?: string;
}

export interface UpdateReleaseNotesRequest {
  notes_text: string;
}

export interface PublishReleaseNotesRequest {
  update_version_record?: boolean;
}

export interface DeprecationInfo {
  contract_id: string;
  status: DeprecationStatus;
  deprecated_at?: string | null;
  retirement_at?: string | null;
  replacement_contract_id?: string | null;
  migration_guide_url?: string | null;
  notes?: string | null;
  days_remaining?: number | null;
  dependents_notified: number;
}

const API_URL = process.env.NEXT_PUBLIC_API_URL || "";
const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === "true";

const CATEGORY_SYNONYMS: Record<string, string> = {
  defi: "DeFi",
  dex: "DeFi",
  lending: "DeFi",
  nft: "NFT",
  governance: "Governance",
  infra: "Infrastructure",
  infrastructure: "Infrastructure",
  payment: "Payment",
  payments: "Payment",
  identity: "Identity",
  game: "Gaming",
  gaming: "Gaming",
  social: "Social",
};

function tokenizeQuery(query: string): string[] {
  return query
    .toLowerCase()
    .replace(/[^\w\s]/g, " ")
    .split(/\s+/)
    .map((token) => token.trim())
    .filter(Boolean);
}

function dedupe<T>(values: T[]): T[] {
  return Array.from(new Set(values));
}

function detectIntent(query: string, params?: ContractSearchParams): SearchIntent {
  const tokens = tokenizeQuery(query);
  const categories = dedupe(
    tokens
      .map((token) => CATEGORY_SYNONYMS[token])
      .filter((value): value is string => Boolean(value)),
  );

  const networks = dedupe(
    tokens
      .map((token) => {
        if (token.includes("mainnet")) return "mainnet";
        if (token.includes("testnet")) return "testnet";
        if (token.includes("futurenet")) return "futurenet";
        return undefined;
      })
      .filter((value): value is Network => Boolean(value)),
  );

  const verifiedOnly =
    tokens.includes("verified") || tokens.includes("audited") || Boolean(params?.verified_only);

  const authorTokenIndex = tokens.findIndex(
    (token) => token === "by" || token === "from" || token === "author",
  );
  const author =
    params?.author ||
    (authorTokenIndex >= 0 && tokens[authorTokenIndex + 1]
      ? tokens[authorTokenIndex + 1]
      : undefined);

  let type: SearchIntentType = "generic";
  if (categories.length > 0) type = "category";
  else if (networks.length > 0) type = "network";
  else if (verifiedOnly) type = "verification";
  else if (author) type = "author";

  const confidence = Math.min(
    0.98,
    0.35 +
      (categories.length > 0 ? 0.2 : 0) +
      (networks.length > 0 ? 0.15 : 0) +
      (verifiedOnly ? 0.15 : 0) +
      (author ? 0.15 : 0),
  );

  return {
    type,
    confidence,
    extracted: {
      categories,
      tags: [],
      networks,
      verified_only: verifiedOnly,
      author,
    },
  };
}

function semanticScore(contract: Contract, queryTokens: string[], intent: SearchIntent): number {
  const haystack = [
    contract.name,
    contract.description || "",
    contract.category || "",
    ...(contract.tags || []),
  ]
    .join(" ")
    .toLowerCase();

  const tokenHits = queryTokens.filter((token) => haystack.includes(token)).length;
  const tokenScore = queryTokens.length > 0 ? tokenHits / queryTokens.length : 0;

  let intentBonus = 0;
  if (intent.type === "category" && intent.extracted.categories.length > 0) {
    const categoryMatch = intent.extracted.categories.some(
      (cat) => contract.category?.toLowerCase() === cat.toLowerCase(),
    );
    intentBonus += categoryMatch ? 0.3 : 0;
  }
  if (intent.type === "network" && intent.extracted.networks.length > 0) {
    intentBonus += intent.extracted.networks.includes(contract.network) ? 0.2 : 0;
  }
  if (intent.type === "verification" && intent.extracted.verified_only) {
    intentBonus += contract.is_verified ? 0.2 : -0.1;
  }

  const popularityBonus = Math.min(0.1, (contract.popularity_score || 0) / 1000);

  return Math.min(1, tokenScore * 0.6 + intentBonus + popularityBonus);
}

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const url = `${API_URL}${path}`;
  let response: Response;
  try {
    response = await fetch(url, {
      headers: { "Content-Type": "application/json", ...(options?.headers || {}) },
      ...options,
    });
  } catch (err) {
    throw new NetworkError(`Network request failed: ${String(err)}`);
  }
  if (!response.ok) {
    const data = await extractErrorData(response);
    throw createApiError(response.status, data);
  }
  return response.json() as Promise<T>;
}

// ─── Contracts ───────────────────────────────────────────────────────────────

export async function fetchContracts(
  params: ContractSearchParams = {},
): Promise<PaginatedResponse<Contract>> {
  if (USE_MOCKS) {
    const {
      query = "",
      page = 1,
      page_size = 20,
      network,
      verified_only,
      category,
      tags,
      sort_by,
      sort_order = "desc",
    } = params;

    let results = [...MOCK_CONTRACTS] as Contract[];

    if (query.trim()) {
      const tokens = tokenizeQuery(query);
      const intent = detectIntent(query, params);
      results = results
        .map((c) => ({ ...c, relevance_score: semanticScore(c, tokens, intent) }))
        .filter((c) => (c.relevance_score || 0) > 0.05)
        .sort((a, b) => (b.relevance_score || 0) - (a.relevance_score || 0));
    }

    if (network) results = results.filter((c) => c.network === network);
    if (verified_only) results = results.filter((c) => c.is_verified);
    if (category) results = results.filter((c) => c.category === category);
    if (tags && tags.length > 0)
      results = results.filter((c) => tags.some((t) => c.tags?.includes(t)));

    if (sort_by && !query.trim()) {
      results.sort((a, b) => {
        let aVal: number | string = 0;
        let bVal: number | string = 0;
        if (sort_by === "name") { aVal = a.name; bVal = b.name; }
        else if (sort_by === "created_at") { aVal = a.created_at; bVal = b.created_at; }
        else if (sort_by === "updated_at") { aVal = a.updated_at; bVal = b.updated_at; }
        else if (sort_by === "popularity") { aVal = a.popularity_score || 0; bVal = b.popularity_score || 0; }
        else if (sort_by === "deployments") { aVal = a.deployment_count || 0; bVal = b.deployment_count || 0; }
        else if (sort_by === "interactions") { aVal = a.interaction_count || 0; bVal = b.interaction_count || 0; }
        else if (sort_by === "downloads") { aVal = a.downloads || 0; bVal = b.downloads || 0; }
        else if (sort_by === "rating") { aVal = a.avg_rating || 0; bVal = b.avg_rating || 0; }
        if (typeof aVal === "string" && typeof bVal === "string") {
          return sort_order === "asc" ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
        }
        return sort_order === "asc"
          ? (aVal as number) - (bVal as number)
          : (bVal as number) - (aVal as number);
      });
    }

    const total = results.length;
    const start = (page - 1) * page_size;
    const items = results.slice(start, start + page_size);
    return {
      items,
      total,
      page,
      page_size,
      total_pages: Math.ceil(total / page_size),
    };
  }

  const searchParams = new URLSearchParams();
  if (params.query) searchParams.set("query", params.query);
  if (params.network) searchParams.set("network", params.network);
  if (params.verified_only) searchParams.set("verified_only", "true");
  if (params.category) searchParams.set("category", params.category);
  if (params.tags?.length) params.tags.forEach((t) => searchParams.append("tags", t));
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  if (params.sort_by) searchParams.set("sort_by", params.sort_by);
  if (params.sort_order) searchParams.set("sort_order", params.sort_order);

  return apiFetch<PaginatedResponse<Contract>>(`/contracts?${searchParams.toString()}`);
}

export async function fetchContract(id: string, network?: Network): Promise<ContractGetResponse> {
  if (USE_MOCKS) {
    const contract = MOCK_CONTRACTS.find(
      (c) => c.id === id || c.contract_id === id,
    ) as ContractGetResponse | undefined;
    if (!contract) throw new ApiError(404, "Contract not found");
    return { ...contract, current_network: network };
  }
  const qs = network ? `?network=${network}` : "";
  return apiFetch<ContractGetResponse>(`/contracts/${id}${qs}`);
}

export async function fetchContractHealth(id: string): Promise<ContractHealth> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      status: "healthy",
      last_activity: new Date().toISOString(),
      security_score: 85,
      total_score: 85,
      recommendations: [],
      updated_at: new Date().toISOString(),
    };
  }
  return apiFetch<ContractHealth>(`/contracts/${id}/health`);
}

export async function fetchContractAnalytics(id: string): Promise<ContractAnalyticsResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      deployments: { count: 0, unique_users: 0, by_network: {} },
      interactors: { unique_count: 0, top_users: [] },
      timeline: [],
    };
  }
  return apiFetch<ContractAnalyticsResponse>(`/contracts/${id}/analytics`);
}

export async function fetchContractVersions(id: string): Promise<ContractVersion[]> {
  if (USE_MOCKS) {
    return (MOCK_VERSIONS[id] || []) as ContractVersion[];
  }
  return apiFetch<ContractVersion[]>(`/contracts/${id}/versions`);
}

export async function fetchContractAbi(id: string, version?: string): Promise<ContractAbiResponse> {
  if (USE_MOCKS) {
    return { abi: null };
  }
  const qs = version ? `?version=${version}` : "";
  return apiFetch<ContractAbiResponse>(`/contracts/${id}/abi${qs}`);
}

export async function fetchContractChangelog(id: string): Promise<ContractChangelogResponse> {
  if (USE_MOCKS) {
    return { contract_id: id, entries: [] };
  }
  return apiFetch<ContractChangelogResponse>(`/contracts/${id}/changelog`);
}

export async function fetchContractRecommendations(
  id: string,
): Promise<ContractRecommendationsResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      algorithm: "mock",
      ab_variant: "a",
      cached: false,
      generated_at: new Date().toISOString(),
      recommendations: [],
    };
  }
  return apiFetch<ContractRecommendationsResponse>(`/contracts/${id}/recommendations`);
}

export async function fetchContractInteractions(
  id: string,
  queryParams: InteractionsQueryParams = {},
): Promise<InteractionsListResponse> {
  if (USE_MOCKS) {
    return { items: [], total: 0, limit: queryParams.limit || 20, offset: queryParams.offset || 0 };
  }
  const searchParams = new URLSearchParams();
  if (queryParams.limit) searchParams.set("limit", String(queryParams.limit));
  if (queryParams.offset) searchParams.set("offset", String(queryParams.offset));
  if (queryParams.account) searchParams.set("account", queryParams.account);
  if (queryParams.method) searchParams.set("method", queryParams.method);
  return apiFetch<InteractionsListResponse>(
    `/contracts/${id}/interactions?${searchParams.toString()}`,
  );
}

export async function publishContract(data: PublishRequest): Promise<Contract> {
  return apiFetch<Contract>("/contracts", {
    method: "POST",
    body: JSON.stringify(data),
  });
}

// ─── Publishers ──────────────────────────────────────────────────────────────

export async function fetchPublisher(id: string): Promise<Publisher> {
  if (USE_MOCKS) {
    return {
      id,
      stellar_address: id,
      created_at: new Date().toISOString(),
    };
  }
  return apiFetch<Publisher>(`/publishers/${id}`);
}

export async function fetchPublishers(
  params: { page?: number; page_size?: number; query?: string } = {},
): Promise<PaginatedResponse<Publisher>> {
  if (USE_MOCKS) {
    return { items: [], total: 0, page: 1, page_size: 20, total_pages: 0 };
  }
  const searchParams = new URLSearchParams();
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  if (params.query) searchParams.set("query", params.query);
  return apiFetch<PaginatedResponse<Publisher>>(`/publishers?${searchParams.toString()}`);
}

export async function fetchPublisherContracts(
  publisherId: string,
  params: ContractSearchParams = {},
): Promise<PaginatedResponse<Contract>> {
  if (USE_MOCKS) {
    const items = MOCK_CONTRACTS.filter(
      (c) => c.publisher_id === publisherId,
    ) as Contract[];
    return {
      items,
      total: items.length,
      page: 1,
      page_size: 20,
      total_pages: Math.ceil(items.length / 20),
    };
  }
  const searchParams = new URLSearchParams();
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  return apiFetch<PaginatedResponse<Contract>>(
    `/publishers/${publisherId}/contracts?${searchParams.toString()}`,
  );
}

// ─── Networks ────────────────────────────────────────────────────────────────

export async function fetchNetworks(): Promise<NetworkListResponse> {
  if (USE_MOCKS) {
    return { networks: [], cached_at: new Date().toISOString() };
  }
  return apiFetch<NetworkListResponse>("/networks");
}

// ─── Search ───────────────────────────────────────────────────────────────────

export async function fetchSearchSuggestions(query: string): Promise<SearchSuggestionsResponse> {
  if (USE_MOCKS || !query.trim()) {
    return { items: [] };
  }
  return apiFetch<SearchSuggestionsResponse>(
    `/search/suggestions?query=${encodeURIComponent(query)}`,
  );
}

export async function semanticSearch(
  params: ContractSearchParams,
): Promise<SemanticContractSearchResponse> {
  if (USE_MOCKS) {
    const base = await fetchContracts(params);
    const intent = detectIntent(params.query || "", params);
    return {
      ...base,
      semantic: {
        raw_query: params.query || "",
        interpreted_query: params.query || "",
        intent,
        fallback_used: false,
        query_suggestions: [],
      },
    };
  }
  const searchParams = new URLSearchParams();
  if (params.query) searchParams.set("query", params.query);
  if (params.network) searchParams.set("network", params.network);
  if (params.verified_only) searchParams.set("verified_only", "true");
  if (params.category) searchParams.set("category", params.category);
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  return apiFetch<SemanticContractSearchResponse>(
    `/search/semantic?${searchParams.toString()}`,
  );
}

// ─── Analytics Activity Feed ──────────────────────────────────────────────────

export async function fetchActivityFeed(
  params: ActivityFeedParams = {},
): Promise<ActivityFeedResponse> {
  if (USE_MOCKS) {
    return { items: [], total: 0, limit: params.limit || 20, next_cursor: null };
  }
  const searchParams = new URLSearchParams();
  if (params.cursor) searchParams.set("cursor", params.cursor);
  if (params.limit) searchParams.set("limit", String(params.limit));
  if (params.event_type) searchParams.set("event_type", params.event_type);
  if (params.contract_id) searchParams.set("contract_id", params.contract_id);
  return apiFetch<ActivityFeedResponse>(`/analytics/activity?${searchParams.toString()}`);
}

// ─── Dependency Graph ─────────────────────────────────────────────────────────

export async function fetchDependencyTree(id: string): Promise<DependencyTreeNode> {
  if (USE_MOCKS) {
    return {
      contract_id: id,
      name: id,
      current_version: "1.0.0",
      constraint_to_parent: "",
      dependencies: [],
    };
  }
  return apiFetch<DependencyTreeNode>(`/contracts/${id}/dependencies/tree`);
}

// ─── Custom Metrics ───────────────────────────────────────────────────────────

export async function fetchMetricCatalog(contractId: string): Promise<MetricCatalogEntry[]> {
  if (USE_MOCKS) return [];
  return apiFetch<MetricCatalogEntry[]>(`/contracts/${contractId}/metrics`);
}

export async function fetchMetricSeries(
  contractId: string,
  metricName: string,
  params: { from?: string; to?: string; resolution?: "hour" | "day" | "raw" } = {},
): Promise<MetricSeriesResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      metric_name: metricName,
      metric_type: null,
      resolution: params.resolution || "day",
      points: [],
    };
  }
  const searchParams = new URLSearchParams();
  if (params.from) searchParams.set("from", params.from);
  if (params.to) searchParams.set("to", params.to);
  if (params.resolution) searchParams.set("resolution", params.resolution);
  return apiFetch<MetricSeriesResponse>(
    `/contracts/${contractId}/metrics/${encodeURIComponent(metricName)}?${searchParams.toString()}`,
  );
}

// ─── Release Notes ────────────────────────────────────────────────────────────

export async function generateReleaseNotes(
  contractId: string,
  data: GenerateReleaseNotesRequest,
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(`/contracts/${contractId}/release-notes/generate`, {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function fetchReleaseNotes(
  contractId: string,
  version: string,
): Promise<ReleaseNotesResponse> {
  if (USE_MOCKS) {
    return {
      id: `${contractId}-${version}`,
      contract_id: contractId,
      version,
      diff_summary: {
        files_changed: 0,
        lines_added: 0,
        lines_removed: 0,
        function_changes: [],
        has_breaking_changes: false,
        features_count: 0,
        fixes_count: 0,
        breaking_count: 0,
      },
      notes_text: "",
      status: "draft",
      generated_by: "mock",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }
  return apiFetch<ReleaseNotesResponse>(`/contracts/${contractId}/release-notes/${version}`);
}

export async function updateReleaseNotes(
  contractId: string,
  version: string,
  data: UpdateReleaseNotesRequest,
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(`/contracts/${contractId}/release-notes/${version}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
}

export async function publishReleaseNotes(
  contractId: string,
  version: string,
  data: PublishReleaseNotesRequest = {},
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(
    `/contracts/${contractId}/release-notes/${version}/publish`,
    { method: "POST", body: JSON.stringify(data) },
  );
}

// ─── Deprecation ──────────────────────────────────────────────────────────────

export async function fetchDeprecationInfo(contractId: string): Promise<DeprecationInfo> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      status: "active",
      dependents_notified: 0,
    };
  }
  return apiFetch<DeprecationInfo>(`/contracts/${contractId}/deprecation`);
}

export async function setDeprecation(
  contractId: string,
  data: Partial<DeprecationInfo>,
): Promise<DeprecationInfo> {
  return apiFetch<DeprecationInfo>(`/contracts/${contractId}/deprecation`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

// ─── Collaborative Review ─────────────────────────────────────────────────────

export async function fetchCollaborativeReview(
  contractId: string,
): Promise<CollaborativeReviewDetails> {
  return apiFetch<CollaborativeReviewDetails>(`/contracts/${contractId}/review`);
}

export async function addReviewComment(
  contractId: string,
  comment: Partial<CollaborativeComment>,
): Promise<CollaborativeComment> {
  return apiFetch<CollaborativeComment>(`/contracts/${contractId}/review/comments`, {
    method: "POST",
    body: JSON.stringify(comment),
  });
}

// ─── Examples ─────────────────────────────────────────────────────────────────

export async function fetchContractExamples(contractId: string): Promise<unknown[]> {
  if (USE_MOCKS) {
    return MOCK_EXAMPLES[contractId] || [];
  }
  return apiFetch<unknown[]>(`/contracts/${contractId}/examples`);
}

// ─── Re-exports ───────────────────────────────────────────────────────────────

export { ApiError, NetworkError } from "./errors";

// ─── Maintenance ──────────────────────────────────────────────────────────────

export async function fetchMaintenanceWindow(): Promise<MaintenanceWindow | null> {
  try {
    return await apiFetch<MaintenanceWindow>("/maintenance");
  } catch {
    return null;
  }
}

// Re-export trackEvent for convenience
export { trackEvent };
