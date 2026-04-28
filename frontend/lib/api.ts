// Mock data: conditionally imported only in development/test.
// In production (NEXT_PUBLIC_USE_MOCKS !== "true"), these are empty stubs
// that never get reached (gated behind USE_MOCKS checks below).
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let MOCK_CONTRACTS: any[] = [];
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let MOCK_EXAMPLES: Record<string, any[]> = {};
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let MOCK_VERSIONS: Record<string, any[]> = {};
if (process.env.NEXT_PUBLIC_USE_MOCKS === "true") {
  // Dynamic require ensures Next.js tree-shakes mock-data from production bundles
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const mocks = require("./mock-data");
  MOCK_CONTRACTS = mocks.MOCK_CONTRACTS;
  MOCK_EXAMPLES = mocks.MOCK_EXAMPLES;
  MOCK_VERSIONS = mocks.MOCK_VERSIONS;
}
import { trackEvent } from "./analytics";
import {
  ApiError,
  NetworkError,
  extractErrorData,
  createApiError,
} from "./errors";
import { resilientCall } from './resilience';
import type { 
  Network, 
  NetworkStatus, 
  NetworkEndpoints, 
  NetworkInfo, 
  NetworkListResponse, 
  NetworkConfig,
  Contract,
  ContractGetResponse,
  ContractHealth,
  ContractInteractionResponse,
  InteractionsQueryParams,
  InteractionsListResponse,
  TimelineEntry,
  TopUser,
  InteractorStats,
  DeploymentStats,
  ContractAnalyticsResponse,
  ContractVersion,
  VersionFieldDiff,
  VersionCompareResponse,
  RevertVersionRequest,
  ContractAbiResponse,
  ContractChangelogEntry,
  ContractChangelogResponse,
  RecommendationReason,
  RecommendedContract,
  ContractRecommendationsResponse,
  CollaborativeReview,
  CollaborativeReviewer,
  CollaborativeComment,
  CollaborativeReviewDetails,
  Publisher,
  AnalyticsEventType,
  AnalyticsEvent,
  ActivityFeedParams,
  ActivityFeedResponse,
  PaginatedResponse,
  DependencyTreeNode,
  MaintenanceWindow,
  MaturityLevel,
  ContractSearchParams,
  SearchSuggestion,
  SearchSuggestionsResponse,
  SearchIntentType,
  SearchIntent,
  SemanticSearchMetadata,
  SemanticContractSearchResponse,
  PublishRequest,
  CustomMetricType,
  MetricCatalogEntry,
  MetricSeriesPoint,
  MetricSample,
  MetricSeriesResponse,
  DeprecationStatus,
  ReleaseNotesStatus,
  FunctionChange,
  DiffSummary,
  ReleaseNotesResponse,
  GenerateReleaseNotesRequest,
  UpdateReleaseNotesRequest,
  PublishReleaseNotesRequest,
  DeprecationInfo
} from "../types";
import type { VerificationLevel } from "../types/verification";


const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";
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

function detectIntent(
  query: string,
  params?: ContractSearchParams,
): SearchIntent {
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
    tokens.includes("verified") ||
    tokens.includes("audited") ||
    Boolean(params?.verified_only);

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

function semanticScore(
  contract: Contract,
  queryTokens: string[],
  intent: SearchIntent,
): number {
  const haystack = [
    contract.name,
    contract.description || "",
    contract.category || "",
    contract.contract_id,
    ...contract.tags,
  ]
    .join(" ")
    .toLowerCase();

  const tokenMatches = queryTokens.reduce(
    (count, token) => (haystack.includes(token) ? count + 1 : count),
    0,
  );
  const tokenCoverage =
    queryTokens.length > 0 ? tokenMatches / queryTokens.length : 0;
  let score = tokenCoverage;

  if (intent.extracted.categories.length > 0 && contract.category) {
    const categoryMatch = intent.extracted.categories.some(
      (category) => category.toLowerCase() === contract.category?.toLowerCase(),
    );
    if (categoryMatch) score += 0.35;
  }

  if (
    intent.extracted.networks.length > 0 &&
    intent.extracted.networks.includes(contract.network)
  ) {
    score += 0.2;
  }

  if (intent.extracted.verified_only && contract.is_verified) {
    score += 0.2;
  }

  return score;
}

function rerankContracts(
  contracts: Contract[],
  query: string,
  intent: SearchIntent,
): Contract[] {
  const tokens = tokenizeQuery(query);
  if (tokens.length === 0) return contracts;
  return [...contracts].sort(
    (a, b) =>
      semanticScore(b, tokens, intent) - semanticScore(a, tokens, intent),
  );
}

function buildSemanticSuggestions(
  query: string,
  intent: SearchIntent,
): string[] {
  const suggestions: string[] = [];
  if (intent.extracted.categories.length === 0) {
    suggestions.push(`${query} DeFi`, `${query} NFT`);
  }
  if (intent.extracted.networks.length === 0) {
    suggestions.push(`${query} on mainnet`);
  }
  if (!intent.extracted.verified_only) {
    suggestions.push(`verified ${query}`);
  }
  return dedupe(suggestions).slice(0, 4);
}

/**
 * Wrapper for API calls with consistent error handling
 */
async function handleApiCall<T>(
  apiCall: () => Promise<Response>,
  endpoint: string,
): Promise<T> {
  // Wrap the API call with a circuit breaker + retries
  try {
    const rawResponse = await resilientCall(endpoint, async () => {
      return apiCall();
    }, { endpoint });

    const response = rawResponse as Response;

    if (!response.ok) {
      const errorData = await extractErrorData(response);
      throw createApiError(response.status, errorData, endpoint);
    }

    try {
      return await response.json();
    } catch (parseError) {
      throw new ApiError(
        "Failed to parse server response",
        response.status,
        parseError,
        endpoint,
      );
    }
  } catch (error) {
    // Re-throw if already an ApiError
    if (error instanceof ApiError) {
      throw error;
    }

    // Circuit open
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    if (error && error.name === 'CircuitOpenError') {
      throw new NetworkError('Service temporarily unavailable (circuit open)', endpoint);
    }

    // Handle network errors
    if (error instanceof TypeError) {
      const message = error.message.toLowerCase();
      if (
        message.includes("fetch") ||
        message.includes("network") ||
        message.includes("failed to fetch")
      ) {
        throw new NetworkError(
          "Unable to connect to the server. Please check your internet connection.",
          endpoint,
        );
      }
    }

    // Handle timeout errors
    if (error instanceof Error && error.name === "AbortError") {
      throw new NetworkError(
        "The request timed out. Please try again.",
        endpoint,
      );
    }

    // Unknown error
    throw new ApiError(
      "An unexpected error occurred",
      undefined,
      error,
      endpoint,
    );
  }
}

export const api = {
  async getNetworks(): Promise<NetworkListResponse> {
    if (USE_MOCKS) {
      const now = new Date().toISOString();
      return {
        cached_at: now,
        networks: [
          {
            id: "mainnet",
            name: "Stellar Mainnet",
            network_type: "mainnet",
            status: "online",
            endpoints: {
              rpc_url: "https://rpc-mainnet.stellar.org",
              health_url: "https://rpc-mainnet.stellar.org/health",
              explorer_url: "https://stellar.expert/explorer/public",
            },
            last_checked_at: now,
            consecutive_failures: 0,
          },
          {
            id: "testnet",
            name: "Stellar Testnet",
            network_type: "testnet",
            status: "online",
            endpoints: {
              rpc_url: "https://rpc-testnet.stellar.org",
              health_url: "https://rpc-testnet.stellar.org/health",
              explorer_url: "https://stellar.expert/explorer/testnet",
              friendbot_url: "https://friendbot.stellar.org",
            },
            last_checked_at: now,
            consecutive_failures: 0,
          },
          {
            id: "futurenet",
            name: "Stellar Futurenet",
            network_type: "futurenet",
            status: "online",
            endpoints: {
              rpc_url: "https://rpc-futurenet.stellar.org",
              health_url: "https://rpc-futurenet.stellar.org/health",
              explorer_url: "https://stellar.expert/explorer/futurenet",
              friendbot_url: "https://friendbot-futurenet.stellar.org",
            },
            last_checked_at: now,
            consecutive_failures: 0,
          },
        ],
      };
    }

    try {
      const resp = await handleApiCall<NetworkListResponse>(
        () => fetch(`${API_URL}/networks`),
        "/networks",
      );
      try {
        // cache for fallback
        if (typeof window !== 'undefined') {
          localStorage.setItem('soroban_cached_networks', JSON.stringify(resp));
        }
      } catch {}
      return resp;
    } catch (err) {
      // On network/circuit failures, fall back to cached networks if available
      try {
        if (typeof window !== 'undefined') {
          const cached = localStorage.getItem('soroban_cached_networks');
          if (cached) return JSON.parse(cached) as NetworkListResponse;
        }
      } catch {}
      throw err;
    }
  },

  // Contract endpoints
  async getContracts(
    params?: ContractSearchParams,
  ): Promise<PaginatedResponse<Contract>> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          let filtered = [...MOCK_CONTRACTS];

          if (params?.query) {
            const q = params.query.toLowerCase();
            filtered = filtered.filter(
              (c) =>
                c.name.toLowerCase().includes(q) ||
                (c.description && c.description.toLowerCase().includes(q)) ||
                c.tags.some((tag: string) => tag.toLowerCase().includes(q)),
            );
          }

          const categories = params?.categories?.length
            ? params.categories
            : params?.category
              ? [params.category]
              : [];
          if (categories.length > 0) {
            filtered = filtered.filter(
              (c) => c.category && categories.includes(c.category),
            );
          }

          const networks = params?.networks?.length
            ? params.networks
            : params?.network
              ? [params.network]
              : [];
          if (networks.length > 0) {
            filtered = filtered.filter((c) => networks.includes(c.network));
          }

          const languages = params?.languages?.length
            ? params.languages
            : params?.language
              ? [params.language]
              : [];
          if (languages.length > 0) {
            const normalized = languages.map((language) =>
              language.toLowerCase(),
            );
            filtered = filtered.filter((c) =>
              c.tags.some((tag: string) =>
                normalized.includes(tag.toLowerCase()),
              ),
            );
          }

          if (params?.author) {
            const author = params.author.toLowerCase();
            filtered = filtered.filter((c) =>
              c.publisher_id.toLowerCase().includes(author),
            );
          }

          if (params?.verified_only) {
            filtered = filtered.filter((c) => c.is_verified);
          }

          if (params?.favorites_only && params.favorites_list) {
            filtered = filtered.filter((c) => params.favorites_list!.includes(c.id));
          }

          if (params?.date_from) {
            const fromTime = new Date(params.date_from).getTime();
            filtered = filtered.filter(
              (c) => new Date(c.created_at).getTime() >= fromTime,
            );
          }
          if (params?.date_to) {
            const toDate = new Date(params.date_to);
            toDate.setUTCHours(23, 59, 59, 999);
            const toTime = toDate.getTime();
            filtered = filtered.filter(
              (c) => new Date(c.created_at).getTime() <= toTime,
            );
          }

          const sortBy = params?.sort_by || "created_at";
          const sortOrder = params?.sort_order || "desc";
          filtered.sort((a, b) => {
            if (sortBy === "name") {
              return a.name.localeCompare(b.name);
            }
            if (sortBy === "popularity") {
              const aPopularity = a.popularity_score ?? 0;
              const bPopularity = b.popularity_score ?? 0;
              return aPopularity - bPopularity;
            }
            if (sortBy === "rating") {
              const aRating = a.average_rating ?? a.avg_rating ?? 0;
              const bRating = b.average_rating ?? b.avg_rating ?? 0;
              if (aRating !== bRating) {
                return aRating - bRating;
              }
              return (a.review_count ?? 0) - (b.review_count ?? 0);
            }
            if (sortBy === "downloads") {
              const aDownloads = a.downloads ?? 0;
              const bDownloads = b.downloads ?? 0;
              return aDownloads - bDownloads;
            }
            return (
              new Date(a.created_at).getTime() -
              new Date(b.created_at).getTime()
            );
          });
          if (sortOrder === "desc") {
            filtered.reverse();
          }

          const page = params?.page || 1;
          const pageSize = params?.page_size || 20;
          const start = (page - 1) * pageSize;
          const end = start + pageSize;
          const items = filtered.slice(start, end);

          resolve({
            items,
            total: filtered.length,
            page,
            page_size: pageSize,
            total_pages: Math.max(1, Math.ceil(filtered.length / pageSize)),
          });
        }, 500);
      });
    }

    const queryParams = new URLSearchParams();
    if (params?.query) queryParams.append("query", params.query);
    if (params?.contract_id) queryParams.append("contract_id", params.contract_id);
    if (params?.network) queryParams.append("network", params.network);
    params?.networks?.forEach((network) => queryParams.append("networks", network));
    if (params?.verified_only !== undefined)
      queryParams.append("verified_only", String(params.verified_only));
    if (params?.category) queryParams.append("category", params.category);
    params?.categories?.forEach((category) => queryParams.append("categories", category));

    try {
      const resp = await handleApiCall<PaginatedResponse<Contract>>(
        () => fetch(`${API_URL}/api/contracts?${queryParams}`),
        "/api/contracts",
      );

      // Normalize legacy field names from older backend responses
      const raw = resp as unknown as Record<string, unknown>;
      const normalized = { ...resp } as PaginatedResponse<Contract> & Record<string, unknown>;
      try {
        if (typeof window !== 'undefined') {
          localStorage.setItem('soroban_cached_contracts', JSON.stringify(normalized));
        }
      } catch {}
      if (Array.isArray(raw.contracts) && !Array.isArray(raw.items)) {
        normalized.items = raw.contracts as Contract[];
      }
      if (typeof raw.pages === "number" && raw.total_pages === undefined) {
        normalized.total_pages = raw.pages as number;
      }
      return normalized;
    } catch (err) {
      // On failure, attempt to return cached contracts list if available
      try {
        if (typeof window !== 'undefined') {
          const cached = localStorage.getItem('soroban_cached_contracts');
          if (cached) return JSON.parse(cached) as PaginatedResponse<Contract>;
        }
      } catch {}
      throw err;
    }
  },

  async semanticSearchContracts(
    params?: ContractSearchParams,
  ): Promise<SemanticContractSearchResponse> {
    const rawQuery = params?.query?.trim() ?? "";
    const intent = detectIntent(rawQuery, params);
    const semanticParams: ContractSearchParams = {
      ...params,
      categories:
        params?.categories && params.categories.length > 0
          ? params.categories
          : intent.extracted.categories.length > 0
            ? intent.extracted.categories
            : params?.categories,
      networks:
        params?.networks && params.networks.length > 0
          ? params.networks
          : intent.extracted.networks.length > 0
            ? intent.extracted.networks
            : params?.networks,
      verified_only: params?.verified_only ?? intent.extracted.verified_only,
      author: params?.author ?? intent.extracted.author,
      query: rawQuery,
    };

    const semanticResult = await api.getContracts(semanticParams);
    let fallbackUsed = false;
    let finalResult = semanticResult;
    const shouldFallback =
      rawQuery.length > 0 && semanticResult.items.length === 0;

    if (shouldFallback) {
      fallbackUsed = true;
      finalResult = await api.getContracts({ ...params, query: rawQuery });
    }

    const rerankedItems = rerankContracts(finalResult.items, rawQuery, intent);
    return {
      ...finalResult,
      items: rerankedItems,
      semantic: {
        raw_query: rawQuery,
        interpreted_query: rawQuery,
        intent,
        fallback_used: fallbackUsed,
        query_suggestions: buildSemanticSuggestions(rawQuery, intent),
      },
    };
  },

  async getContractSearchSuggestions(
    q: string,
    limit = 8,
  ): Promise<SearchSuggestionsResponse> {
    if (USE_MOCKS) {
      const lowered = q.trim().toLowerCase();
      if (!lowered) {
        return { items: [] };
      }

      const items = Array.from(
        new Set(
          MOCK_CONTRACTS.flatMap((contract) =>
            [contract.name, contract.category].filter(Boolean),
          ).filter((value: string) => value.toLowerCase().includes(lowered)),
        ),
      )
        .slice(0, limit)
        .map((text) => ({
          text,
          kind: MOCK_CONTRACTS.some((contract) => contract.name === text)
            ? "contract"
            : "category",
          score: 1,
        }));

      return { items };
    }

    const params = new URLSearchParams();
    params.set("q", q);
    params.set("limit", String(limit));

    return handleApiCall<SearchSuggestionsResponse>(
      () => fetch(`${API_URL}/api/contracts/suggestions?${params.toString()}`),
      "/api/contracts/suggestions",
    );
  },

  async getContract(
    id: string,
    network?: Network,
  ): Promise<ContractGetResponse> {
    if (USE_MOCKS) {
      return new Promise((resolve, reject) => {
        setTimeout(() => {
          const contract = MOCK_CONTRACTS.find(
            (c) => c.id === id || c.contract_id === id,
          );
          if (contract) {
            resolve(contract as ContractGetResponse);
          } else {
            reject(new Error("Contract not found"));
          }
        }, 300);
      });
    }

    return handleApiCall<Contract>(() => {
      const url = new URL(`${API_URL}/api/contracts/${id}`);
      if (network != null) url.searchParams.set("network", String(network));
      return fetch(url.toString());
    }, `/api/contracts/${id}`);
  },

  async getContractExamples(id: string): Promise<ContractExample[]> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve(MOCK_EXAMPLES[id] || []);
        }, 500);
      });
    }

    return handleApiCall<ContractExample[]>(
      () => fetch(`${API_URL}/api/contracts/${id}/examples`),
      `/api/contracts/${id}/examples`,
    );
  },

  async rateExample(
    id: string,
    userAddress: string,
    rating: number,
  ): Promise<ExampleRating> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve({
            id: "mock-rating-id",
            example_id: id,
            user_address: userAddress,
            rating: rating,
            created_at: new Date().toISOString(),
          });
        }, 300);
      });
    }

    return handleApiCall<ExampleRating>(
      () =>
        fetch(`${API_URL}/api/examples/${id}/rate`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ user_address: userAddress, rating }),
        }),
      `/api/examples/${id}/rate`,
    );
  },

  async getContractVersions(id: string): Promise<ContractVersion[]> {
    if (USE_MOCKS) {
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve(MOCK_VERSIONS[id] || []);
        }, 300);
      });
    }

    return handleApiCall<ContractVersion[]>(
      () => fetch(`${API_URL}/api/contracts/${id}/versions`),
      `/api/contracts/${id}/versions`,
    );
  },

  async getContractAbi(
    id: string,
    version?: string,
  ): Promise<ContractAbiResponse> {
    const url = new URL(`${API_URL}/api/contracts/${id}/abi`);
    if (version) url.searchParams.set("version", version);
    return handleApiCall<ContractAbiResponse>(
      () => fetch(url.toString()),
      `/api/contracts/${id}/abi`,
    );
  },

  async getContractChangelog(id: string): Promise<ContractChangelogResponse> {
    return handleApiCall<ContractChangelogResponse>(
      () => fetch(`${API_URL}/api/contracts/${id}/changelog`),
      `/api/contracts/${id}/changelog`,
    );
  },

  async compareContractVersions(
    id: string,
    from: string,
    to: string,
  ): Promise<VersionCompareResponse> {
    const url = new URL(`${API_URL}/api/contracts/${id}/versions/compare`);
    url.searchParams.set("from", from);
    url.searchParams.set("to", to);

    return handleApiCall<VersionCompareResponse>(
      () => fetch(url.toString()),
      `/api/contracts/${id}/versions/compare`,
    );
  },

  async revertContractVersion(
    id: string,
    version: string,
    payload: RevertVersionRequest,
  ): Promise<ContractVersion> {
    return handleApiCall<ContractVersion>(
      () =>
        fetch(
          `${API_URL}/api/admin/contracts/${id}/versions/${version}/revert`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload),
          },
        ),
      `/api/admin/contracts/${id}/versions/${version}/revert`,
    );
  },

  async getContractDependencies(id: string): Promise<DependencyTreeNode[]> {
    return handleApiCall<DependencyTreeNode[]>(
      () => fetch(`${API_URL}/api/contracts/${id}/dependencies`),
      `/api/contracts/${id}/dependencies`,
    );
  },

  async getContractLocalGraph(
    id: string,
    depth?: number,
  ): Promise<GraphResponse> {
    const search = new URLSearchParams();
    if (depth != null) search.set("depth", String(depth));
    const qs = search.toString();
    return handleApiCall<GraphResponse>(
      () => fetch(`${API_URL}/api/contracts/${id}/graph${qs ? `?${qs}` : ""}`),
      `/api/contracts/${id}/graph`,
    );
  },

  async getContractInteractions(
    id: string,
    params?: InteractionsQueryParams,
  ): Promise<InteractionsListResponse> {
    const search = new URLSearchParams();
    if (params?.limit != null) search.set("limit", String(params.limit));
    if (params?.offset != null) search.set("offset", String(params.offset));
    if (params?.account) search.set("account", params.account);
    if (params?.method) search.set("method", params.method);
    if (params?.from_timestamp)
      search.set("from_timestamp", params.from_timestamp);
    if (params?.to_timestamp) search.set("to_timestamp", params.to_timestamp);
    const qs = search.toString();
    const response = await fetch(
      `${API_URL}/api/contracts/${id}/interactions${qs ? `?${qs}` : ""}`,
    );
    if (!response.ok) throw new Error("Failed to fetch contract interactions");
    return response.json();
  },

  async getContractAnalytics(id: string): Promise<ContractAnalyticsResponse> {
    const response = await fetch(`${API_URL}/api/contracts/${id}/analytics`);
    if (!response.ok) throw new Error("Failed to fetch contract analytics");
    return response.json();
  },

  async getActivityFeed(
    params?: ActivityFeedParams,
  ): Promise<ActivityFeedResponse> {
    if (USE_MOCKS) {
      // Basic mock for activity feed
      const items: AnalyticsEvent[] = [
        {
          id: "1",
          event_type: "contract_published",
          contract_id: "C123...",
          user_address: "G...123",
          network: "testnet",
          metadata: { name: "SorobanToken" },
          created_at: new Date().toISOString(),
        },
        {
          id: "2",
          event_type: "contract_verified",
          contract_id: "C456...",
          user_address: "G...456",
          network: "mainnet",
          metadata: { name: "BridgeContract" },
          created_at: new Date(Date.now() - 3600000).toISOString(),
        },
      ];
      return {
        items,
        total: items.length,
        limit: params?.limit ?? 20,
        next_cursor: null,
      };
    }

    const search = new URLSearchParams();
    if (params?.cursor) search.set("cursor", params.cursor);
    if (params?.limit != null) search.set("limit", String(params.limit));
    if (params?.event_type) search.set("event_type", params.event_type);
    if (params?.contract_id) search.set("contract_id", params.contract_id);

    const qs = search.toString();
    return handleApiCall<ActivityFeedResponse>(
      () => fetch(`${API_URL}/api/activity-feed${qs ? `?${qs}` : ""}`),
      "/api/activity-feed",
    );
  },

  async getContractRecommendations(
    id: string,
    params?: {
      limit?: number;
      network?: Network;
      subject?: string;
      algorithm?: "hybrid_v1" | "hybrid_v2";
    },
  ): Promise<ContractRecommendationsResponse> {
    const search = new URLSearchParams();
    if (params?.limit != null) search.set("limit", String(params.limit));
    if (params?.network) search.set("network", params.network);
    if (params?.subject) search.set("subject", params.subject);
    if (params?.algorithm) search.set("algorithm", params.algorithm);

    return handleApiCall<ContractRecommendationsResponse>(
      () =>
        fetch(
          `${API_URL}/api/contracts/${id}/recommendations${search.toString() ? `?${search.toString()}` : ""}`,
        ),
      `/api/contracts/${id}/recommendations`,
    );
  },

  async publishContract(data: PublishRequest): Promise<Contract> {
    if (USE_MOCKS) {
      if (typeof window !== "undefined") {
        trackEvent("contract_publish_failed", {
          network: data.network,
          name: data.name,
          reason: "mock_mode_not_supported",
        });
      }
      throw new Error("Publishing is not supported in mock mode");
    }

    try {
      const response = await fetch(`${API_URL}/api/contracts`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data),
      });
      if (!response.ok) throw new Error("Failed to publish contract");

      const published = await response.json();
      if (typeof window !== "undefined") {
        trackEvent("contract_published", {
          contract_id: data.contract_id,
          name: data.name,
          network: data.network,
          category: data.category,
        });
      }

      return published;
    } catch (error) {
      if (typeof window !== "undefined") {
        trackEvent("contract_publish_failed", {
          contract_id: data.contract_id,
          name: data.name,
          network: data.network,
        });
      }
      throw error;
    }
  },

  async getContractHealth(id: string): Promise<ContractHealth> {
    return handleApiCall<ContractHealth>(
      () => fetch(`${API_URL}/api/contracts/${id}/health`),
      `/api/contracts/${id}/health`,
    );
  },

  async getDeprecationInfo(id: string): Promise<DeprecationInfo> {
    if (USE_MOCKS) {
      return Promise.resolve({
        contract_id: id,
        status: "deprecated",
        deprecated_at: new Date(Date.now() - 86400000 * 7).toISOString(),
        retirement_at: new Date(Date.now() + 86400000 * 30).toISOString(),
        replacement_contract_id: "c2",
        migration_guide_url: "https://example.com/migration",
        notes:
          "This contract is being retired. Migrate to the new liquidity pool contract.",
        days_remaining: 30,
        dependents_notified: 4,
      });
    }

    return handleApiCall<DeprecationInfo>(
      () => fetch(`${API_URL}/api/contracts/${id}/deprecation-info`),
      `/api/contracts/${id}/deprecation-info`,
    );
  },

  async getFormalVerificationResults(
    id: string,
  ): Promise<FormalVerificationReport[]> {
    if (USE_MOCKS) {
      return Promise.resolve([]);
    }
    return handleApiCall<FormalVerificationReport[]>(
      () => fetch(`${API_URL}/api/contracts/${id}/formal-verification`),
      `/api/contracts/${id}/formal-verification`,
    );
  },

  async runFormalVerification(
    id: string,
    data: RunVerificationRequest,
  ): Promise<FormalVerificationReport> {
    if (USE_MOCKS) {
      throw new Error("Formal verification is not supported in mock mode");
    }
    return handleApiCall<FormalVerificationReport>(
      () =>
        fetch(`${API_URL}/api/contracts/${id}/formal-verification`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(data),
        }),
      `/api/contracts/${id}/formal-verification`,
    );
  },

  async getCustomMetricCatalog(id: string): Promise<MetricCatalogEntry[]> {
    if (USE_MOCKS) {
      return Promise.resolve([
        {
          metric_name: "custom_trades_volume",
          metric_type: "counter",
          last_seen: new Date().toISOString(),
          sample_count: 128,
        },
        {
          metric_name: "custom_liquidity_depth",
          metric_type: "gauge",
          last_seen: new Date().toISOString(),
          sample_count: 72,
        },
      ]);
    }

    const response = await fetch(
      `${API_URL}/api/contracts/${id}/metrics/catalog`,
    );
    if (!response.ok) throw new Error("Failed to fetch metrics catalog");
    return response.json();
  },

  async getCustomMetricSeries(
    id: string,
    metric: string,
    options?: {
      resolution?: "hour" | "day" | "raw";
      from?: string;
      to?: string;
      limit?: number;
    },
  ): Promise<MetricSeriesResponse> {
    if (USE_MOCKS) {
      const now = Date.now();
      const points = Array.from({ length: 24 }).map((_, idx) => {
        const bucketStart = new Date(now - (23 - idx) * 3600_000).toISOString();
        const bucketEnd = new Date(now - (22 - idx) * 3600_000).toISOString();
        return {
          bucket_start: bucketStart,
          bucket_end: bucketEnd,
          sample_count: 12,
          avg_value: Math.random() * 1000,
          p95_value: Math.random() * 1200,
          max_value: Math.random() * 1500,
          sum_value: Math.random() * 5000,
        } satisfies MetricSeriesPoint;
      });

      return Promise.resolve({
        contract_id: id,
        metric_name: metric,
        metric_type: "counter",
        resolution: options?.resolution ?? "hour",
        points,
      });
    }

    const queryParams = new URLSearchParams();
    queryParams.append("metric", metric);
    if (options?.resolution)
      queryParams.append("resolution", options.resolution);
    if (options?.from) queryParams.append("from", options.from);
    if (options?.to) queryParams.append("to", options.to);
    if (options?.limit) queryParams.append("limit", String(options.limit));

    const response = await fetch(
      `${API_URL}/api/contracts/${id}/metrics?${queryParams.toString()}`,
    );
    if (!response.ok) throw new Error("Failed to fetch metric series");
    return response.json();
  },

  // Publisher endpoints
  async getPublisher(id: string): Promise<Publisher> {
    if (USE_MOCKS) {
      return Promise.resolve({
        id: id,
        stellar_address: "G...",
        username: "Mock Publisher",
        created_at: new Date().toISOString(),
      });
    }

    return handleApiCall<Publisher>(
      () => fetch(`${API_URL}/api/publishers/${id}`),
      `/api/publishers/${id}`,
    );
  },

  async getPublisherContracts(id: string): Promise<Contract[]> {
    if (USE_MOCKS) {
      return Promise.resolve(
        MOCK_CONTRACTS.filter((c) => c.publisher_id === id),
      );
    }

    return handleApiCall<Contract[]>(
      () => fetch(`${API_URL}/api/publishers/${id}/contracts`),
      `/api/publishers/${id}/contracts`,
    );
  },

  async getStats(): Promise<{
    total_contracts: number;
    verified_contracts: number;
    total_publishers: number;
  }> {
    if (USE_MOCKS) {
      return Promise.resolve({
        total_contracts: MOCK_CONTRACTS.length,
        verified_contracts: MOCK_CONTRACTS.filter((c) => c.is_verified).length,
        total_publishers: 5,
      });
    }

    return handleApiCall<{
      total_contracts: number;
      verified_contracts: number;
      total_publishers: number;
    }>(() => fetch(`${API_URL}/api/stats`), "/api/stats");
  },

  // Version upgrade compatibility endpoint
  async getCompatibility(id: string): Promise<CompatibilityMatrix> {
    return handleApiCall<CompatibilityMatrix>(
      () => fetch(`${API_URL}/api/contracts/${id}/compatibility`),
      `/api/contracts/${id}/compatibility`,
    );
  },

  async getInteroperability(
    id: string,
  ): Promise<ContractInteroperabilityResponse> {
    return handleApiCall<ContractInteroperabilityResponse>(
      () => fetch(`${API_URL}/api/contracts/${id}/interoperability`),
      `/api/contracts/${id}/interoperability`,
    );
  },

  async addCompatibility(
    id: string,
    data: AddCompatibilityRequest,
  ): Promise<unknown> {
    return handleApiCall<unknown>(
      () =>
        fetch(`${API_URL}/api/contracts/${id}/compatibility`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(data),
        }),
      `/api/contracts/${id}/compatibility`,
    );
  },

  getCompatibilityExportUrl(id: string, format: "csv" | "json"): string {
    return `${API_URL}/api/contracts/${id}/compatibility/export?format=${format}`;
  },

  // Graph endpoint (backend may return { graph: {} } or { nodes, edges }; normalize to GraphResponse)
  async getContractGraph(network?: string): Promise<GraphResponse> {
    const queryParams = new URLSearchParams();
    if (network) queryParams.append("network", network);
    const qs = queryParams.toString();
    return handleApiCall<GraphResponse>(
      () => fetch(`${API_URL}/api/contracts/graph${qs ? `?${qs}` : ""}`),
      "/api/contracts/graph",
    );
  },

  async getTemplates(): Promise<Template[]> {
    if (USE_MOCKS) {
      return Promise.resolve([]);
    }
    return handleApiCall<Template[]>(
      () => fetch(`${API_URL}/api/templates`),
      "/api/templates",
    );
  },

  // SDK / Wasm / Network Compatibility Testing (Issue #261)
  async getCompatibilityMatrix(
    id: string,
  ): Promise<CompatibilityTestMatrixResponse> {
    return handleApiCall<CompatibilityTestMatrixResponse>(
      () => fetch(`${API_URL}/api/contracts/${id}/compatibility-matrix`),
      `/api/contracts/${id}/compatibility-matrix`,
    );
  },

  async runCompatibilityTest(
    id: string,
    data: RunCompatibilityTestRequest,
  ): Promise<CompatibilityTestEntry> {
    return handleApiCall<CompatibilityTestEntry>(
      () =>
        fetch(`${API_URL}/api/contracts/${id}/compatibility-matrix/test`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(data),
        }),
      `/api/contracts/${id}/compatibility-matrix/test`,
    );
  },

  async getCompatibilityHistory(
    id: string,
    limit?: number,
    offset?: number,
  ): Promise<CompatibilityHistoryResponse> {
    const params = new URLSearchParams();
    if (limit != null) params.set("limit", String(limit));
    if (offset != null) params.set("offset", String(offset));
    const qs = params.toString();
    return handleApiCall<CompatibilityHistoryResponse>(
      () =>
        fetch(
          `${API_URL}/api/contracts/${id}/compatibility-matrix/history${qs ? `?${qs}` : ""}`,
        ),
      `/api/contracts/${id}/compatibility-matrix/history`,
    );
  },

  async getCompatibilityNotifications(
    id: string,
  ): Promise<CompatibilityNotification[]> {
    return handleApiCall<CompatibilityNotification[]>(
      () =>
        fetch(
          `${API_URL}/api/contracts/${id}/compatibility-matrix/notifications`,
        ),
      `/api/contracts/${id}/compatibility-matrix/notifications`,
    );
  },

  async markCompatibilityNotificationsRead(id: string): Promise<unknown> {
    return handleApiCall<unknown>(
      () =>
        fetch(
          `${API_URL}/api/contracts/${id}/compatibility-matrix/notifications/read`,
          {
            method: "POST",
          },
        ),
      `/api/contracts/${id}/compatibility-matrix/notifications/read`,
    );
  },

  async getCompatibilityDashboard(): Promise<CompatibilityDashboardResponse> {
    return handleApiCall<CompatibilityDashboardResponse>(
      () => fetch(`${API_URL}/api/compatibility-dashboard`),
      "/api/compatibility-dashboard",
    );
  },

  // ── Release Notes Generation ────────────────────────────────────────────

  async listReleaseNotes(id: string): Promise<ReleaseNotesResponse[]> {
    return handleApiCall<ReleaseNotesResponse[]>(
      () => fetch(`${API_URL}/api/contracts/${id}/release-notes`),
      `/api/contracts/${id}/release-notes`,
    );
  },

  async getReleaseNotes(
    id: string,
    version: string,
  ): Promise<ReleaseNotesResponse> {
    return handleApiCall<ReleaseNotesResponse>(
      () => fetch(`${API_URL}/api/contracts/${id}/release-notes/${version}`),
      `/api/contracts/${id}/release-notes/${version}`,
    );
  },

  async generateReleaseNotes(
    id: string,
    req: GenerateReleaseNotesRequest,
  ): Promise<ReleaseNotesResponse> {
    return handleApiCall<ReleaseNotesResponse>(
      () =>
        fetch(`${API_URL}/api/contracts/${id}/release-notes/generate`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(req),
        }),
      `/api/contracts/${id}/release-notes/generate`,
    );
  },

  async updateReleaseNotes(
    id: string,
    version: string,
    req: UpdateReleaseNotesRequest,
  ): Promise<ReleaseNotesResponse> {
    return handleApiCall<ReleaseNotesResponse>(
      () =>
        fetch(`${API_URL}/api/contracts/${id}/release-notes/${version}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(req),
        }),
      `/api/contracts/${id}/release-notes/${version}`,
    );
  },

  async publishReleaseNotes(
    id: string,
    version: string,
    req?: PublishReleaseNotesRequest,
  ): Promise<ReleaseNotesResponse> {
    return handleApiCall<ReleaseNotesResponse>(
      () =>
        fetch(
          `${API_URL}/api/contracts/${id}/release-notes/${version}/publish`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(req ?? { update_version_record: true }),
          },
        ),
      `/api/contracts/${id}/release-notes/${version}/publish`,
    );
  },

  // Database Migration Versioning (Issue #252)
  async getMigrationStatus(): Promise<MigrationStatusResponse> {
    return handleApiCall<MigrationStatusResponse>(
      () => fetch(`${API_URL}/api/admin/migrations/status`),
      "/api/admin/migrations/status",
    );
  },

  async registerMigration(
    data: RegisterMigrationRequest,
  ): Promise<RegisterMigrationResponse> {
    return handleApiCall<RegisterMigrationResponse>(
      () =>
        fetch(`${API_URL}/api/admin/migrations/register`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(data),
        }),
      "/api/admin/migrations/register",
    );
  },

  async validateMigrations(): Promise<MigrationValidationResponse> {
    return handleApiCall<MigrationValidationResponse>(
      () => fetch(`${API_URL}/api/admin/migrations/validate`),
      "/api/admin/migrations/validate",
    );
  },

  async getMigrationLockStatus(): Promise<LockStatusResponse> {
    return handleApiCall<LockStatusResponse>(
      () => fetch(`${API_URL}/api/admin/migrations/lock`),
      "/api/admin/migrations/lock",
    );
  },

  async getMigrationVersion(version: number): Promise<SchemaVersion> {
    return handleApiCall<SchemaVersion>(
      () => fetch(`${API_URL}/api/admin/migrations/${version}`),
      `/api/admin/migrations/${version}`,
    );
  },

  async rollbackMigration(version: number): Promise<RollbackResponse> {
    return handleApiCall<RollbackResponse>(
      () =>
        fetch(`${API_URL}/api/admin/migrations/${version}/rollback`, {
          method: "POST",
        }),
      `/api/admin/migrations/${version}/rollback`,
    );
  },

  // Advanced Search (Issue #51)
  async advancedSearchContracts(
    req: AdvancedSearchRequest,
  ): Promise<PaginatedResponse<Contract>> {
    return handleApiCall<PaginatedResponse<Contract>>(
      () =>
        fetch(`${API_URL}/api/contracts/search`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(req),
        }),
      "/api/contracts/search",
    );
  },

  async listFavoriteSearches(): Promise<FavoriteSearch[]> {
    return handleApiCall<FavoriteSearch[]>(
      () => fetch(`${API_URL}/api/favorites/search`),
      "/api/favorites/search",
    );
  },

  async saveFavoriteSearch(
    req: SaveFavoriteSearchRequest,
  ): Promise<FavoriteSearch> {
    return handleApiCall<FavoriteSearch>(
      () =>
        fetch(`${API_URL}/api/favorites/search`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(req),
        }),
      "/api/favorites/search",
    );
  },

  async deleteFavoriteSearch(id: string): Promise<void> {
    const response = await fetch(`${API_URL}/api/favorites/search/${id}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      throw new Error(
        `Failed to delete favorite search: ${response.statusText}`,
      );
    }
  },

  // ── Contract Comments / Discussion (Issue #516) ───────────────────────────

  async getComments(contractId: string): Promise<CommentListResponse> {
    if (USE_MOCKS || typeof window !== "undefined") {
      return Promise.resolve(getLocalComments(contractId));
    }
    return handleApiCall<CommentListResponse>(
      () => fetch(`${API_URL}/api/contracts/${contractId}/comments`),
      `/api/contracts/${contractId}/comments`,
    );
  },

  async postComment(
    contractId: string,
    body: string,
    parentId?: string,
  ): Promise<Comment> {
    if (USE_MOCKS || typeof window !== "undefined") {
      const comment: Comment = {
        id: `local-${Date.now()}`,
        contract_id: contractId,
        parent_id: parentId ?? null,
        author: "You",
        body,
        created_at: new Date().toISOString(),
        score: 0,
        flagged: false,
        flag_count: 0,
      };
      const stored = getLocalComments(contractId);
      stored.items.unshift(comment);
      stored.total += 1;
      setLocalComments(contractId, stored);
      return Promise.resolve(comment);
    }
    return handleApiCall<Comment>(
      () =>
        fetch(`${API_URL}/api/contracts/${contractId}/comments`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ body, parent_id: parentId }),
        }),
      `/api/contracts/${contractId}/comments`,
    );
  },

  async voteComment(
    commentId: string,
    contractId: string,
    direction: "up" | "down",
  ): Promise<CommentVote> {
    if (USE_MOCKS || typeof window !== "undefined") {
      const stored = getLocalComments(contractId);
      stored.items = stored.items.map((c) =>
        c.id === commentId
          ? { ...c, score: c.score + (direction === "up" ? 1 : -1) }
          : c,
      );
      setLocalComments(contractId, stored);
      return Promise.resolve({ comment_id: commentId, direction });
    }
    return handleApiCall<CommentVote>(
      () =>
        fetch(`${API_URL}/api/comments/${commentId}/vote`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ direction }),
        }),
      `/api/comments/${commentId}/vote`,
    );
  },

  async flagComment(
    commentId: string,
    contractId: string,
    reason: string,
  ): Promise<CommentFlag> {
    if (USE_MOCKS || typeof window !== "undefined") {
      const stored = getLocalComments(contractId);
      stored.items = stored.items.map((c) =>
        c.id === commentId
          ? { ...c, flagged: true, flag_count: c.flag_count + 1 }
          : c,
      );
      setLocalComments(contractId, stored);
      return Promise.resolve({ comment_id: commentId, reason });
    }
    return handleApiCall<CommentFlag>(
      () =>
        fetch(`${API_URL}/api/comments/${commentId}/flag`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ reason }),
        }),
      `/api/comments/${commentId}/flag`,
    );
  },

  // ── Favorites preferences (authenticated) ──────────────────────────────

  async getPreferences(token: string): Promise<UserFavoritesPreferences> {
    return handleApiCall<UserFavoritesPreferences>(
      () =>
        fetch(`${API_URL}/api/me/preferences`, {
          headers: { Authorization: `Bearer ${token}` },
        }),
      "/api/me/preferences",
    );
  },

  async updatePreferences(
    token: string,
    favorites: string[],
  ): Promise<UserFavoritesPreferences> {
    return handleApiCall<UserFavoritesPreferences>(
      () =>
        fetch(`${API_URL}/api/me/preferences`, {
          method: "PATCH",
          headers: {
            Authorization: `Bearer ${token}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ favorites }),
        }),
      "/api/me/preferences",
    );
  },
};

export interface Template {
  id: string;
  slug: string;
  name: string;
  description?: string;
  category: string;
  version: string;
  install_count: number;
  // Image fields for template icon/thumbnail
  thumbnail_url?: string;
  parameters: {
    name: string;
    type: string;
    default?: string;
    description?: string;
  }[];
  created_at: string;
}

export interface GraphNode {
  id: string;
  contract_id: string;
  name: string;
  network: "mainnet" | "testnet" | "futurenet";
  is_verified: boolean;
  category?: string;
  tags: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
  dependency_type: string;
  call_frequency?: number;
  call_volume?: number;
  is_estimated?: boolean;
  is_circular?: boolean;
}

export interface GraphResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface ContractExample {
  id: string;
  contract_id: string;
  title: string;
  description?: string;
  code_rust?: string;
  code_js?: string;
  category?: "basic" | "advanced" | "integration";
  rating_up: number;
  rating_down: number;
  created_at: string;
  updated_at: string;
}

export interface ExampleRating {
  id: string;
  example_id: string;
  user_address: string;
  rating: number;
  created_at: string;
}

export type ProtocolComplianceStatus = "compliant" | "partial" | "unsupported";

export type InteroperabilityCapabilityKind = "bridge" | "adapter";

export interface InteroperabilityProtocolMatch {
  slug: string;
  name: string;
  description: string;
  status: ProtocolComplianceStatus;
  matched_functions: string[];
  missing_functions: string[];
  optional_matches: string[];
  compliance_score: number;
}

export interface InteroperabilityCapability {
  kind: InteroperabilityCapabilityKind;
  label: string;
  confidence: number;
  evidence: string[];
}

export interface InteroperabilitySuggestion {
  contract_id: string;
  contract_address: string;
  contract_name: string;
  network: Network;
  category?: string;
  is_verified: boolean;
  score: number;
  reason: string;
  shared_protocols: string[];
  shared_functions: string[];
  relation_types: string[];
}

export interface InteroperabilitySummary {
  protocol_matches: number;
  compatible_contracts: number;
  suggested_contracts: number;
  graph_nodes: number;
  graph_edges: number;
  bridge_signals: number;
  adapter_signals: number;
}

export interface ContractInteroperabilityResponse {
  contract_id: string;
  contract_address: string;
  contract_name: string;
  network: Network;
  analyzed_at: string;
  has_abi: boolean;
  analyzed_functions: string[];
  warnings: string[];
  protocols: InteroperabilityProtocolMatch[];
  capabilities: InteroperabilityCapability[];
  suggestions: InteroperabilitySuggestion[];
  graph: GraphResponse;
  summary: InteroperabilitySummary;
}

// ─── Compatibility Matrix ────────────────────────────────────────────────────

export interface CompatibilityEntry {
  target_version: string;
  has_breaking_changes: boolean;
  breaking_changes: string[];
  breaking_change_count: number;
  stellar_version?: string;
  is_compatible: boolean;
}

export interface CompatibilityRow {
  source_version: string;
  targets: CompatibilityEntry[];
}

export interface CompatibilityMatrix {
  contract_id: string;
  contract_stellar_id: string;
  version_order: string[];
  rows: CompatibilityRow[];
  warnings: string[];
  total_pairs: number;
}

export interface AddCompatibilityRequest {
  source_version: string;
  target_contract_id: string;
  target_version: string;
  stellar_version?: string;
  is_compatible: boolean;
}

// ─── SDK / Wasm / Network Compatibility Testing (Issue #261) ─────────────────

export type CompatibilityTestStatus = "compatible" | "warning" | "incompatible";

export interface CompatibilityTestEntry {
  sdk_version: string;
  wasm_runtime: string;
  network: string;
  status: CompatibilityTestStatus;
  tested_at: string;
  test_duration_ms?: number;
  error_message?: string;
}

export interface CompatibilityTestSummary {
  total_tests: number;
  compatible_count: number;
  warning_count: number;
  incompatible_count: number;
}

export interface CompatibilityTestMatrixResponse {
  contract_id: string;
  sdk_versions: string[];
  wasm_runtimes: string[];
  networks: string[];
  entries: CompatibilityTestEntry[];
  summary: CompatibilityTestSummary;
  last_tested?: string;
}

export interface RunCompatibilityTestRequest {
  sdk_version: string;
  wasm_runtime: string;
  network: string;
}

export interface CompatibilityHistoryEntry {
  id: string;
  contract_id: string;
  sdk_version: string;
  wasm_runtime: string;
  network: string;
  previous_status?: CompatibilityTestStatus;
  new_status: CompatibilityTestStatus;
  changed_at: string;
  change_reason?: string;
}

export interface CompatibilityHistoryResponse {
  contract_id: string;
  changes: CompatibilityHistoryEntry[];
  total: number;
}

export interface CompatibilityNotification {
  id: string;
  contract_id: string;
  sdk_version: string;
  message: string;
  is_read: boolean;
  created_at: string;
}

export interface CompatibilityDashboardResponse {
  total_contracts_tested: number;
  overall_compatible: number;
  overall_warning: number;
  overall_incompatible: number;
  sdk_versions: string[];
  recent_changes: CompatibilityHistoryEntry[];
}

// ─── Database Migration Versioning (Issue #252) ──────────────────────────────

export interface SchemaVersion {
  id: number;
  version: number;
  description: string;
  filename: string;
  checksum: string;
  applied_at: string;
  applied_by: string;
  execution_time_ms?: number;
  rolled_back_at?: string;
  rollback_by?: string;
}

export interface MigrationStatusResponse {
  current_version?: number;
  total_applied: number;
  total_rolled_back: number;
  pending_count: number;
  versions: SchemaVersion[];
  has_lock: boolean;
  healthy: boolean;
  warnings: string[];
}

export interface ChecksumMismatch {
  version: number;
  filename: string;
  expected_checksum: string;
  actual_checksum: string;
}

export interface MigrationValidationResponse {
  valid: boolean;
  mismatches: ChecksumMismatch[];
  missing: number[];
}

export interface RegisterMigrationRequest {
  version: number;
  description: string;
  filename: string;
  sql_content: string;
  down_sql?: string;
}

export interface RegisterMigrationResponse {
  version: number;
  checksum: string;
  message: string;
}

export interface RollbackResponse {
  version: number;
  rolled_back_at: string;
  message: string;
}

export interface LockStatusResponse {
  locked: boolean;
  locked_by?: string;
  locked_at?: string;
}

// ─── Formal Verification ─────────────────────────────────────────────────────

export type VerificationStatus = "Proved" | "Violated" | "Unknown" | "Skipped";

export interface FormalVerificationSession {
  id: string;
  contract_id: string;
  version: string;
  verifier_version: string;
  created_at: string;
  updated_at: string;
}

export interface FormalVerificationProperty {
  id: string;
  session_id: string;
  property_id: string;
  description?: string;
  invariant: string;
  severity: string;
}

export interface FormalVerificationResult {
  id: string;
  property_id: string;
  status: VerificationStatus;
  counterexample?: string;
  details?: string;
}

export interface FormalVerificationPropertyResult {
  property: FormalVerificationProperty;
  result: FormalVerificationResult;
}

export interface FormalVerificationReport {
  session: FormalVerificationSession;

  properties: FormalVerificationPropertyResult[];
}

export interface RunVerificationRequest {
  properties_file: string;
  verifier_version?: string;
}

// ─── Advanced Search & Favorites (Issue #51) ─────────────────────────────────

export type QueryOperator = "AND" | "OR";
export type FieldOperator =
  | "eq"
  | "ne"
  | "gt"
  | "lt"
  | "in"
  | "contains"
  | "starts_with";

export interface QueryCondition {
  field: string;
  operator: FieldOperator;
  value: string | number | boolean | string[];
}

export type QueryNode =
  | QueryCondition
  | { operator: QueryOperator; conditions: QueryNode[] };

export interface AdvancedSearchRequest {
  query: QueryNode;
  sort_by?: ContractSearchParams["sort_by"];
  sort_order?: ContractSearchParams["sort_order"];
  limit?: number;
  offset?: number;
}

export interface FavoriteSearch {
  id: string;
  user_id?: string;
  name: string;
  query_json: QueryNode;
  created_at: string;
  updated_at: string;
}

export interface UserFavoritesPreferences {
  favorites: string[];
}

export interface SaveFavoriteSearchRequest {
  name: string;
  query: QueryNode;
}

// ─── Comment / Discussion (Issue #516) ───────────────────────────────────────

export interface Comment {
  id: string;
  contract_id: string;
  parent_id: string | null;
  author: string;
  body: string;
  created_at: string;
  score: number;
  flagged: boolean;
  flag_count: number;
}

export interface CommentVote {
  comment_id: string;
  direction: "up" | "down";
}

export interface CommentFlag {
  comment_id: string;
  reason: string;
}

export interface CommentListResponse {
  items: Comment[];
  total: number;
}

const COMMENT_STORAGE_PREFIX = "soroban_comments_";

function seedComments(contractId: string): CommentListResponse {
  const now = new Date();
  const older = new Date(now.getTime() - 1000 * 60 * 60 * 24).toISOString();
  const root: Comment = {
    id: "seed-1",
    contract_id: contractId,
    parent_id: null,
    author: "GDRXE7BFEBOWQ3BHPNFTUOBCIGGKCGJPNIDZWNOSIROWKJZTIVWY5WYP",
    body: "Great contract. Works well with the token factory. One thing to note: calling `transfer` with a zero amount will silently succeed rather than returning an error.",
    created_at: older,
    score: 4,
    flagged: false,
    flag_count: 0,
  };
  const reply: Comment = {
    id: "seed-2",
    contract_id: contractId,
    parent_id: "seed-1",
    author: "GCO2IP3MJNUOKS4PUDI4C7LGGMQDJGXG3COYX3WSB4HHNAHKYV5YL3VC",
    body: "Confirmed. Also worth checking the `allowance` return value before calling `transfer_from` — the ABI says `i128` but the error is opaque when allowance is exceeded.",
    created_at: now.toISOString(),
    score: 2,
    flagged: false,
    flag_count: 0,
  };
  return { items: [root, reply], total: 2 };
}

function getLocalComments(contractId: string): CommentListResponse {
  if (typeof window === "undefined") return { items: [], total: 0 };
  const key = `${COMMENT_STORAGE_PREFIX}${contractId}`;
  const raw = window.localStorage.getItem(key);
  if (!raw) {
    const seeded = seedComments(contractId);
    window.localStorage.setItem(key, JSON.stringify(seeded));
    return seeded;
  }
  try {
    return JSON.parse(raw) as CommentListResponse;
  } catch {
    return { items: [], total: 0 };
  }
}

function setLocalComments(contractId: string, data: CommentListResponse): void {
  if (typeof window === "undefined") return;
  const key = `${COMMENT_STORAGE_PREFIX}${contractId}`;
  window.localStorage.setItem(key, JSON.stringify(data));
}
