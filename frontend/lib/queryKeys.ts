/**
 * Every React Query cache key the app uses, in one place.
 *
 * Before this module the keys were written inline at each call site, so the
 * same query was spelled out in up to eight files and a mutation invalidated a
 * query by re-typing its string. A typo there does not fail loudly — the
 * invalidation simply matches nothing and the screen keeps showing stale data.
 *
 * The arrays produced here are exactly the ones that were inline before, so the
 * cache layout and every prefix-match invalidation behave as they did. In
 * particular `contracts()` is a prefix of both `contractsList()` and
 * `contractsRecent()`, which is what lets a publish invalidate every contract
 * list at once.
 */

import type { InteractionsQueryParams } from "@/types";

/**
 * An id as a call site actually has it. Several of these queries run on a page
 * whose route param has not resolved yet and are guarded with `enabled`, so the
 * id reaching the key can be undefined or null — as it could before this module
 * existed. Keeping that in the type stops the widening from being silent.
 */
type QueryId = string | undefined | null;

export const queryKeys = {
  // Contracts
  contracts: () => ["contracts"] as const,
  contractsList: (params: unknown) => ["contracts", params] as const,
  contractsRecent: () => ["contracts", "recent"] as const,
  contract: (id: QueryId) => ["contract", id] as const,
  contractAbi: (id: QueryId, version?: string) =>
    ["contract-abi", id, version] as const,
  contractAnalytics: (id: QueryId) => ["contract-analytics", id] as const,
  contractAnalyticsSummary: (id: QueryId) =>
    ["contract-analytics-summary", id] as const,
  contractChangelog: (id: QueryId) => ["contract-changelog", id] as const,
  contractComments: (id: QueryId) => ["contract-comments", id] as const,
  contractDependencies: (id: QueryId) => ["contract-dependencies", id] as const,
  contractDeprecation: (id: QueryId) => ["contract-deprecation", id] as const,
  contractExamples: (id: QueryId) => ["contract-examples", id] as const,
  contractHealth: (id: QueryId) => ["health", id] as const,
  contractInteractions: (id: QueryId, params?: InteractionsQueryParams) =>
    params === undefined
      ? (["contract-interactions", id] as const)
      : (["contract-interactions", id, params] as const),
  contractSource: (id: QueryId, sourceUrl?: string) =>
    ["contract-source", id, sourceUrl] as const,
  contractTimeline: (id: QueryId) => ["contract-timeline", id] as const,
  contractVersions: (id: QueryId) => ["contract-versions", id] as const,
  contractQuickView: (id: QueryId) => ["contract-quick-view", id] as const,
  contractQuickViewAbi: (id: QueryId) => ["contract-quick-view-abi", id] as const,

  // Graph
  contractGraph: (network?: string) => ["contract-graph", network] as const,
  contractLocalGraph: (id: QueryId, depth?: number) =>
    ["contract-local-graph", id, depth] as const,

  // Compatibility
  compatibilityMatrix: (id: QueryId) => ["compatibility-matrix", id] as const,
  compatibilityHistory: (id: QueryId) => ["compatibility-history", id] as const,
  compatibilityNotifications: (id: QueryId) =>
    ["compatibility-notifications", id] as const,
  /** The version-compatibility view, which derives its matrix from versions. */
  versionCompatibility: (id: QueryId) =>
    ["compatibility", id] as const,

  // Reviews and comparison
  collaborativeReview: (id: QueryId) => ["collaborative-review", id] as const,
  compareSearch: (query: string) =>
    ["compare", "contracts-search", query] as const,
  compareSelected: (ids: string[]) =>
    ["compare", "selected-contracts", ids] as const,
  diffSource: (id: QueryId, version: string | undefined) =>
    ["diff-source", id, version] as const,

  // Everything else
  activityFeed: (eventType?: string) => ["activity-feed", eventType] as const,
  customMetricsCatalog: (id: QueryId) =>
    ["custom-metrics-catalog", id] as const,
  customMetricsSeries: (id: QueryId, metric: string, resolution: string) =>
    ["custom-metrics-series", id, metric, resolution] as const,
  dependencyScan: (id: QueryId) => ["dependency-scan", id] as const,
  favoriteSearches: () => ["favorite-searches"] as const,
  formalVerification: (id: QueryId) => ["formal-verification", id] as const,
  publisher: (address: QueryId) => ["publisher", address] as const,
  releaseNotes: (id: QueryId) => ["release-notes", id] as const,
  stats: () => ["stats"] as const,
  templates: () => ["templates"] as const,
} as const;

export type QueryKeys = typeof queryKeys;
