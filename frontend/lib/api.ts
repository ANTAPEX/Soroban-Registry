import type {
  ActivityFeedParams,
  ActivityFeedResponse,
  AnalyticsEvent,
  CollaborativeReview,
  Comment,
  CommentListResponse,
  CompatibilityHistoryResponse,
  CompatibilityNotification,
  CompatibilityTestEntry,
  CompatibilityTestMatrixResponse,
  Contract,
  ContractAbiResponse,
  ContractAnalyticsResponse,
  ContractChangelogResponse,
  ContractExample,
  ContractGetResponse,
  ContractHealth,
  ContractRecommendationsResponse,
  ContractSearchParams,
  ContractVersion,
  CreateCollaborativeReviewRequest,
  DependencyScanReport,
  DependencyTreeNode,
  DeprecationInfo,
  FavoriteSearch,
  FormalVerificationReport,
  GenerateReleaseNotesRequest,
  GraphResponse,
  InteractionsListResponse,
  InteractionsQueryParams,
  LegacyStatsResponse,
  MaintenanceWindow,
  MetricCatalogEntry,
  MetricSeriesResponse,
  Network,
  NetworkListResponse,
  PackageDependency,
  PackageDependencyInput,
  PaginatedResponse,
  PublishReleaseNotesRequest,
  PublishRequest,
  Publisher,
  QueryNode,
  ReleaseNotesResponse,
  RunCompatibilityTestRequest,
  SearchIntent,
  SearchIntentType,
  SearchSuggestionsResponse,
  SemanticContractSearchResponse,
  Template,
  UpdateReleaseNotesRequest,
  UserPreferences,
} from "@/types";
import {
  CollaborativeComment,
  CollaborativeReviewDetails,
  TimePeriod,
} from "@/types";

// Every domain type lives in `types/`. They are re-exported here so that
// existing `from "@/lib/api"` imports keep resolving.
export type {
  ActivityFeedParams,
  ActivityFeedResponse,
  AnalyticsEvent,
  AnalyticsEventType,
  CollaborativeReview,
  Comment,
  CommentListResponse,
  CompatibilityEntry,
  CompatibilityHistoryEntry,
  CompatibilityHistoryResponse,
  CompatibilityMatrix,
  CompatibilityMatrixRow,
  CompatibilityNotification,
  CompatibilityTestEntry,
  CompatibilityTestMatrixResponse,
  CompatibilityTestStatus,
  CompatibilityTestSummary,
  Contract,
  ContractAbiResponse,
  ContractAnalyticsResponse,
  ContractChangelogEntry,
  ContractChangelogResponse,
  ContractExample,
  ContractGetResponse,
  ContractHealth,
  ContractInteractionResponse,
  ContractInteroperabilityResponse,
  ContractRecommendationsResponse,
  ContractSearchParams,
  ContractVersion,
  CreateCollaborativeReviewRequest,
  CustomMetricType,
  DependencyScanReport,
  DependencyScanStatus,
  DependencyTreeNode,
  DependencyVulnerabilityFinding,
  DeploymentStats,
  DeprecationInfo,
  DeprecationStatus,
  DiffSummary,
  FavoriteSearch,
  FormalVerificationFinding,
  FormalVerificationProperty,
  FormalVerificationPropertyDefinition,
  FormalVerificationPropertyResult,
  FormalVerificationReport,
  FormalVerificationSession,
  FunctionChange,
  GenerateReleaseNotesRequest,
  GraphEdge,
  GraphNode,
  GraphResponse,
  InteractionsListResponse,
  InteractionsQueryParams,
  InteractorStats,
  InteroperabilityCapability,
  InteroperabilityCapabilityKind,
  InteroperabilityProtocolMatch,
  InteroperabilitySuggestion,
  InteroperabilitySummary,
  LegacyStatsResponse,
  MaintenanceWindow,
  MaturityLevel,
  MetricCatalogEntry,
  MetricSample,
  MetricSeriesPoint,
  MetricSeriesResponse,
  Network,
  NetworkConfig,
  NetworkEndpoints,
  NetworkInfo,
  NetworkListResponse,
  NetworkStatus,
  PackageDependency,
  PackageDependencyInput,
  PaginatedResponse,
  ProofCertificate,
  PublishReleaseNotesRequest,
  PublishRequest,
  Publisher,
  QueryNode,
  RecommendationReason,
  RecommendedContract,
  ReleaseNotesResponse,
  ReleaseNotesStatus,
  RunCompatibilityTestRequest,
  SearchIntent,
  SearchIntentType,
  SearchSuggestion,
  SearchSuggestionsResponse,
  SemanticContractSearchResponse,
  SemanticSearchMetadata,
  Template,
  TemplateParameter,
  TimelineEntry,
  TopUser,
  UpdateReleaseNotesRequest,
  UserPreferences,
} from "@/types";
import { trackEvent } from "./analytics";
import { fetchStats } from "./api/stats";
import { ApiError } from "./errors";
import { fetchAnalytics } from "./api/analytics";
import { apiFetch } from "./api/client";
import { API_URL, USE_MOCKS } from "@/lib/env";

// Mock data: conditionally imported only in development/test.
// In production (NEXT_PUBLIC_USE_MOCKS !== "true"), these are empty stubs
// that never get reached (gated behind USE_MOCKS checks below).
/* eslint-disable @typescript-eslint/no-explicit-any */
let MOCK_CONTRACTS: any[] = [];
let MOCK_EXAMPLES: Record<string, any[]> = {};
let MOCK_VERSIONS: Record<string, any[]> = {};
/* eslint-enable @typescript-eslint/no-explicit-any */
if (USE_MOCKS) {
  // Dynamic require ensures Next.js tree-shakes mock-data from production bundles
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const mocks = require("./mock-data");
  MOCK_CONTRACTS = mocks.MOCK_CONTRACTS;
  MOCK_EXAMPLES = mocks.MOCK_EXAMPLES;
  MOCK_VERSIONS = mocks.MOCK_VERSIONS;
}










































































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
      networks,
      verified_only,
      category,
      categories,
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
    if (networks?.length) results = results.filter((c) => networks.includes(c.network));
    if (verified_only) results = results.filter((c) => c.is_verified);
    if (category) results = results.filter((c) => c.category === category);
    if (categories?.length) results = results.filter((c) => categories.includes(c.category ?? ""));
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
  if (params.networks?.length) {
    params.networks.forEach((n) => searchParams.append("networks", n));
  }
  if (params.verified_only) searchParams.set("verified_only", "true");
  if (params.category) searchParams.set("category", params.category);
  if (params.categories?.length) {
    params.categories.forEach((c) => searchParams.append("categories", c));
  }
  if (params.tags?.length) params.tags.forEach((t) => searchParams.append("tags", t));
  if (params.page) searchParams.set("page", String(params.page));
  if (params.page_size) searchParams.set("page_size", String(params.page_size));
  if (params.sort_by) searchParams.set("sort_by", params.sort_by);
  if (params.sort_order) searchParams.set("sort_order", params.sort_order);

  return apiFetch<PaginatedResponse<Contract>>(`/api/contracts?${searchParams.toString()}`);
}

export async function advancedSearchContracts(
  params: {
    query: QueryNode;
    limit?: number;
    offset?: number;
    sort_by?: ContractSearchParams["sort_by"];
    sort_order?: ContractSearchParams["sort_order"];
  },
): Promise<PaginatedResponse<Contract>> {
  return apiFetch<PaginatedResponse<Contract>>("/api/contracts/search", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

export async function fetchContract(id: string, network?: Network): Promise<ContractGetResponse> {
  if (USE_MOCKS) {
    const contract = MOCK_CONTRACTS.find(
      (c) => c.id === id || c.contract_id === id,
    ) as ContractGetResponse | undefined;
    if (!contract) throw new ApiError("Contract not found", 404);
    return { ...contract, current_network: network };
  }
  const qs = network ? `?network=${network}` : "";
  return apiFetch<ContractGetResponse>(`/api/contracts/${id}${qs}`);
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
  return apiFetch<ContractHealth>(`/api/contracts/${id}/health`);
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
  return apiFetch<ContractAnalyticsResponse>(`/api/contracts/${id}/analytics`);
}

export async function fetchContractVersions(id: string): Promise<ContractVersion[]> {
  if (USE_MOCKS) {
    return (MOCK_VERSIONS[id] || []) as ContractVersion[];
  }
  return apiFetch<ContractVersion[]>(`/api/contracts/${id}/versions`);
}

export async function fetchContractAbi(id: string, version?: string): Promise<ContractAbiResponse> {
  if (USE_MOCKS) {
    return { abi: null };
  }
  const qs = version ? `?version=${version}` : "";
  return apiFetch<ContractAbiResponse>(`/api/contracts/${id}/abi${qs}`);
}

export async function fetchContractChangelog(id: string): Promise<ContractChangelogResponse> {
  if (USE_MOCKS) {
    return { contract_id: id, entries: [] };
  }
  return apiFetch<ContractChangelogResponse>(`/api/contracts/${id}/changelog`);
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
  return apiFetch<ContractRecommendationsResponse>(`/api/contracts/${id}/recommendations`);
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
    `/api/contracts/${id}/interactions?${searchParams.toString()}`,
  );
}

export async function publishContract(data: PublishRequest): Promise<Contract> {
  return apiFetch<Contract>("/api/contracts", {
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
  return apiFetch<Publisher>(`/api/publishers/${id}`);
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
  return apiFetch<PaginatedResponse<Publisher>>(`/api/publishers?${searchParams.toString()}`);
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
    `/api/publishers/${publisherId}/contracts?${searchParams.toString()}`,
  );
}

// ─── Networks ────────────────────────────────────────────────────────────────

export async function fetchNetworks(): Promise<NetworkListResponse> {
  if (USE_MOCKS) {
    return { networks: [], cached_at: new Date().toISOString() };
  }
  return apiFetch<NetworkListResponse>("/api/networks");
}

// ─── Search ───────────────────────────────────────────────────────────────────

export async function fetchSearchSuggestions(query: string): Promise<SearchSuggestionsResponse> {
  if (USE_MOCKS || !query.trim()) {
    return { items: [] };
  }
  return apiFetch<SearchSuggestionsResponse>(
    `/api/search/suggestions?query=${encodeURIComponent(query)}`,
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
    `/api/search/semantic?${searchParams.toString()}`,
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
  // Backend returns a `CursorPaginatedResponse<AnalyticsEvent>` shaped as
  // { data, total, has_more, next_cursor } — adapt it to the `items`/`limit`
  // shape the rest of the frontend expects.
  const raw = await apiFetch<{
    data: AnalyticsEvent[];
    total: number;
    has_more: boolean;
    next_cursor: string | null;
  }>(`/api/activity-feed?${searchParams.toString()}`);
  return {
    items: raw.data,
    total: raw.total,
    limit: params.limit || 20,
    next_cursor: raw.next_cursor,
  };
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
  return apiFetch<DependencyTreeNode>(`/api/contracts/${id}/dependencies/tree`);
}

// ─── Custom Metrics ───────────────────────────────────────────────────────────

export async function fetchMetricCatalog(contractId: string): Promise<MetricCatalogEntry[]> {
  if (USE_MOCKS) return [];
  return apiFetch<MetricCatalogEntry[]>(`/api/contracts/${contractId}/metrics`);
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
    `/api/contracts/${contractId}/metrics/${encodeURIComponent(metricName)}?${searchParams.toString()}`,
  );
}

// ─── Release Notes ────────────────────────────────────────────────────────────

export async function generateReleaseNotes(
  contractId: string,
  data: GenerateReleaseNotesRequest,
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(`/api/contracts/${contractId}/release-notes/generate`, {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function listReleaseNotes(contractId: string): Promise<ReleaseNotesResponse[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<ReleaseNotesResponse[]>(`/api/contracts/${contractId}/release-notes`);
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
  return apiFetch<ReleaseNotesResponse>(`/api/contracts/${contractId}/release-notes/${version}`);
}

export async function updateReleaseNotes(
  contractId: string,
  version: string,
  data: UpdateReleaseNotesRequest,
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(`/api/contracts/${contractId}/release-notes/${version}`, {
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
    `/api/contracts/${contractId}/release-notes/${version}/publish`,
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
  return apiFetch<DeprecationInfo>(`/api/contracts/${contractId}/deprecation`);
}

export async function setDeprecation(
  contractId: string,
  data: Partial<DeprecationInfo>,
): Promise<DeprecationInfo> {
  return apiFetch<DeprecationInfo>(`/api/contracts/${contractId}/deprecation`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

// ─── Templates ────────────────────────────────────────────────────────────────

export async function fetchTemplates(): Promise<Template[]> {
  if (USE_MOCKS) return Promise.resolve([]);
  return apiFetch<Template[]>("/api/templates");
}

// ─── Contract Graph ───────────────────────────────────────────────────────────

export async function fetchContractGraph(network?: Network | string): Promise<GraphResponse> {
  if (USE_MOCKS) {
    return { nodes: [], edges: [] };
  }
  const qs = network ? `?network=${network}` : "";
  return apiFetch<GraphResponse>(`/api/contracts/graph${qs}`);
}

export async function fetchContractLocalGraph(
  contractId: string,
  depth?: number,
): Promise<GraphResponse> {
  if (USE_MOCKS) {
    return { nodes: [], edges: [] };
  }
  const search = new URLSearchParams();
  if (depth != null) search.set("depth", String(depth));
  const qs = search.toString() ? `?${search.toString()}` : "";
  return apiFetch<GraphResponse>(`/api/contracts/${contractId}/graph${qs}`);
}

// ─── Formal Verification ──────────────────────────────────────────────────────

export async function fetchFormalVerificationResults(
  contractId: string,
): Promise<FormalVerificationReport[]> {
  if (USE_MOCKS) {
    return [];
  }
  const listResponse = await apiFetch<{
    items: Array<{ id: string }>;
    total: number;
  }>(`/api/contracts/${contractId}/formal-verification`);

  if (!listResponse.items.length) {
    return [];
  }

  const latestSession = listResponse.items[0];
  const detail = await apiFetch<FormalVerificationReport>(
    `/api/contracts/${contractId}/formal-verification/${latestSession.id}`,
  );
  return [detail];
}

// ─── Dependency Vulnerability Scanning ────────────────────────────────────────

export async function fetchDependencyScanReport(
  contractId: string,
): Promise<DependencyScanReport> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      status: "not_scanned",
      dependencies_scanned: 0,
      vulnerable_dependency_count: 0,
      last_scanned_at: null,
      findings: [],
    };
  }
  return apiFetch<DependencyScanReport>(
    `/api/contracts/${contractId}/dependency-scan`,
  );
}

export async function triggerDependencyScan(
  contractId: string,
): Promise<DependencyScanReport> {
  if (USE_MOCKS) {
    return fetchDependencyScanReport(contractId);
  }
  return apiFetch<DependencyScanReport>(
    `/api/contracts/${contractId}/dependency-scan`,
    { method: "POST" },
  );
}

export async function fetchPackageDependencies(
  contractId: string,
): Promise<PackageDependency[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<PackageDependency[]>(
    `/api/contracts/${contractId}/package-dependencies`,
  );
}

export async function declarePackageDependencies(
  contractId: string,
  dependencies: PackageDependencyInput[],
): Promise<DependencyScanReport> {
  return apiFetch<DependencyScanReport>(
    `/api/contracts/${contractId}/package-dependencies`,
    {
      method: "POST",
      body: JSON.stringify({ dependencies }),
    },
  );
}

// ─── Compatibility Testing ────────────────────────────────────────────────────

export async function fetchCompatibilityMatrix(
  contractId: string,
): Promise<CompatibilityTestMatrixResponse> {
  if (USE_MOCKS) {
    return {
      contract_id: contractId,
      sdk_versions: [],
      wasm_runtimes: [],
      networks: [],
      entries: [],
      summary: {
        total_tests: 0,
        compatible_count: 0,
        warning_count: 0,
        incompatible_count: 0,
      },
      last_tested: null,
    };
  }
  return apiFetch<CompatibilityTestMatrixResponse>(`/api/contracts/${contractId}/compatibility-matrix`);
}

export async function runCompatibilityTest(
  contractId: string,
  request: RunCompatibilityTestRequest,
): Promise<CompatibilityTestEntry> {
  return apiFetch<CompatibilityTestEntry>(`/api/contracts/${contractId}/compatibility-matrix/test`, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export async function fetchCompatibilityHistory(
  contractId: string,
  limit?: number,
  offset?: number,
): Promise<CompatibilityHistoryResponse> {
  if (USE_MOCKS) {
    return { contract_id: contractId, changes: [], total: 0 };
  }
  const search = new URLSearchParams();
  if (limit != null) search.set("limit", String(limit));
  if (offset != null) search.set("offset", String(offset));
  const qs = search.toString() ? `?${search.toString()}` : "";
  return apiFetch<CompatibilityHistoryResponse>(`/api/contracts/${contractId}/compatibility-matrix/history${qs}`);
}

export async function fetchCompatibilityNotifications(
  contractId: string,
): Promise<CompatibilityNotification[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<CompatibilityNotification[]>(`/api/contracts/${contractId}/compatibility-matrix/notifications`);
}

export function getCompatibilityExportUrl(
  contractId: string,
  format: "csv" | "json",
): string {
  return `${API_URL}/api/contracts/${contractId}/compatibility-matrix/export?format=${format}`;
}

// ─── Comments ─────────────────────────────────────────────────────────────────

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

export async function listFavoriteSearches(): Promise<FavoriteSearch[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<FavoriteSearch[]>("/api/favorites/search");
}

export async function deleteFavoriteSearch(id: string): Promise<void> {
  await apiFetch<void>(`/api/favorites/search/${id}`, {
    method: "DELETE",
  });
}

// ─── Preferences ──────────────────────────────────────────────────────────────


export async function fetchPreferences(token: string): Promise<UserPreferences> {
  if (USE_MOCKS) {
    return { favorites: [] };
  }
  return apiFetch<UserPreferences>("/api/me/preferences", {
    headers: { Authorization: `Bearer ${token}` },
  });
}

export async function updatePreferences(
  token: string,
  favorites: string[],
): Promise<UserPreferences> {
  if (USE_MOCKS) {
    return { favorites };
  }
  return apiFetch<UserPreferences>("/api/me/preferences", {
    method: "PATCH",
    headers: { Authorization: `Bearer ${token}` },
    body: JSON.stringify({ favorites }),
  });
}

// ─── Contract Search Suggestions ───────────────────────────────────────────────

export async function fetchContractSearchSuggestions(
  query: string,
  limit?: number,
): Promise<SearchSuggestionsResponse> {
  if (USE_MOCKS || !query.trim()) {
    return { items: [] };
  }
  const search = new URLSearchParams();
  search.set("query", query);
  if (limit != null) search.set("limit", String(limit));
  return apiFetch<SearchSuggestionsResponse>(`/api/search/suggestions?${search.toString()}`);
}

// ─── Custom Metrics ───────────────────────────────────────────────────────────

export async function fetchCustomMetricCatalog(contractId: string): Promise<MetricCatalogEntry[]> {
  if (USE_MOCKS) return [];
  return apiFetch<MetricCatalogEntry[]>(`/api/contracts/${contractId}/metrics/catalog`);
}

export async function fetchCustomMetricSeries(
  contractId: string,
  metricName: string,
  params: { from?: string; to?: string; resolution?: "hour" | "day" | "raw"; limit?: number } = {},
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
  if (params.limit) searchParams.set("limit", String(params.limit));
  return apiFetch<MetricSeriesResponse>(
    `/api/contracts/${contractId}/metrics/${encodeURIComponent(metricName)}?${searchParams.toString()}`,
  );
}

// ─── Collaborative Review ─────────────────────────────────────────────────────

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

// ─── Examples ─────────────────────────────────────────────────────────────────

export async function fetchContractExamples(contractId: string): Promise<ContractExample[]> {
  if (USE_MOCKS) {
    return MOCK_EXAMPLES[contractId] || [];
  }
  return apiFetch<ContractExample[]>(`/api/contracts/${contractId}/examples`);
}

export async function rateExample(
  exampleId: string,
  userId: string,
  rating: number,
): Promise<void> {
  await apiFetch<void>(`/api/examples/${exampleId}/rating`, {
    method: "POST",
    body: JSON.stringify({ user_id: userId, rating }),
  });
}

// ─── Re-exports ───────────────────────────────────────────────────────────────

export { ApiError, NetworkError } from "./errors";

// ─── Maintenance ──────────────────────────────────────────────────────────────

export async function fetchMaintenanceWindow(): Promise<MaintenanceWindow | null> {
  try {
    return await apiFetch<MaintenanceWindow>("/api/maintenance");
  } catch {
    return null;
  }
}

export async function getStats(
  period: TimePeriod = "all-time",
): Promise<LegacyStatsResponse> {
  const response = await fetch(`${API_URL}/api/stats?period=${encodeURIComponent(period)}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch stats: ${response.status}`);
  }

  const rawStats = (await response.json()) as {
    total_contracts: number;
    verified_contracts: number;
    total_publishers: number;
  };

  return {
    total_contracts: rawStats.total_contracts,
    verified_contracts: rawStats.verified_contracts,
    total_publishers: rawStats.total_publishers,
    totalContracts: rawStats.total_contracts,
    verifiedPercentage:
      rawStats.total_contracts > 0
        ? (rawStats.verified_contracts / rawStats.total_contracts) * 100
        : 0,
    totalPublishers: rawStats.total_publishers,
    networkBreakdown: [],
    contractsByCategory: [],
    deploymentsTrend: [],
    topPublishers: [],
  };
}

// Re-export trackEvent for convenience
export { trackEvent };

// ─── api namespace object ─────────────────────────────────────────────────────
// Provides `import { api } from "@/lib/api"` compatibility used across components.

export const api = {
  fetchContracts,
  getContracts: fetchContracts,
  advancedSearchContracts,
  fetchContract,
  getContract: fetchContract,
  fetchContractHealth,
  getContractHealth: fetchContractHealth,
  fetchContractAnalytics,
  getContractAnalytics: fetchContractAnalytics,
  fetchContractVersions,
  getContractVersions: fetchContractVersions,
  fetchContractAbi,
  fetchAnalytics,
  getStats,
  fetchStats,
  getContractAbi: fetchContractAbi,
  fetchContractChangelog,
  getContractChangelog: fetchContractChangelog,
  fetchContractRecommendations,
  getContractRecommendations: fetchContractRecommendations,
  fetchContractInteractions,
  getContractInteractions: fetchContractInteractions,
  publishContract,
  fetchPublisher,
  getPublisher: fetchPublisher,
  fetchPublishers,
  getPublishers: fetchPublishers,
  fetchPublisherContracts,
  getPublisherContracts: fetchPublisherContracts,
  fetchNetworks,
  getNetworks: fetchNetworks,
  fetchSearchSuggestions,
  getSearchSuggestions: fetchSearchSuggestions,
  semanticSearch,
  fetchActivityFeed,
  getActivityFeed: fetchActivityFeed,
  fetchDependencyTree,
  getContractDependencies: fetchDependencyTree,
  fetchMetricCatalog,
  getMetricCatalog: fetchMetricCatalog,
  fetchMetricSeries,
  getMetricSeries: fetchMetricSeries,
  generateReleaseNotes,
  listReleaseNotes,
  fetchReleaseNotes,
  getReleaseNotes: fetchReleaseNotes,
  updateReleaseNotes,
  publishReleaseNotes,
  fetchDeprecationInfo,
  getDeprecationInfo: fetchDeprecationInfo,
  setDeprecation,
  fetchCollaborativeReview,
  getCollaborativeReview: fetchCollaborativeReview,
  createCollaborativeReview,
  addReviewComment,
  addCollaborativeComment: addReviewComment,
  updateReviewerStatus,
  fetchContractExamples,
  getContractExamples: fetchContractExamples,
  rateExample,
  fetchMaintenanceWindow,
  getMaintenanceWindow: fetchMaintenanceWindow,
  // Backward-compatible aliases for stats and templates
  fetchTemplates,
  getTemplates: fetchTemplates,
  // Backward-compatible aliases for graph methods
  fetchContractGraph,
  getContractGraph: fetchContractGraph,
  fetchContractLocalGraph,
  getContractLocalGraph: fetchContractLocalGraph,
  // Backward-compatible aliases for formal verification
  fetchFormalVerificationResults,
  getFormalVerificationResults: fetchFormalVerificationResults,
  // Dependency vulnerability scanning
  fetchDependencyScanReport,
  getDependencyScanReport: fetchDependencyScanReport,
  triggerDependencyScan,
  fetchPackageDependencies,
  declarePackageDependencies,
  // Backward-compatible aliases for compatibility testing
  fetchCompatibilityMatrix,
  getCompatibilityMatrix: fetchCompatibilityMatrix,
  runCompatibilityTest,
  fetchCompatibilityHistory,
  getCompatibilityHistory: fetchCompatibilityHistory,
  fetchCompatibilityNotifications,
  getCompatibilityNotifications: fetchCompatibilityNotifications,
  getCompatibilityExportUrl,
  // Backward-compatible aliases for comments
  fetchComments,
  getComments: fetchComments,
  postComment,
  voteComment,
  flagComment,
  listFavoriteSearches,
  deleteFavoriteSearch,
  // Backward-compatible aliases for preferences
  fetchPreferences,
  getPreferences: fetchPreferences,
  updatePreferences,
  // Backward-compatible aliases for search suggestions
  fetchContractSearchSuggestions,
  getContractSearchSuggestions: fetchContractSearchSuggestions,
  // Backward-compatible aliases for custom metrics
  fetchCustomMetricCatalog,
  getCustomMetricCatalog: fetchCustomMetricCatalog,
  fetchCustomMetricSeries,
  getCustomMetricSeries: fetchCustomMetricSeries,
};
