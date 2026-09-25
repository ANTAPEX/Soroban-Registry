import { API_URL, USE_MOCKS } from "@/lib/env";
import type { StatsResponse, TimePeriod, LegacyStatsResponse } from "@/types";


// Raw shapes of the three backend endpoints the stats page combines.
interface RegistryStatsRaw {
  total_contracts: number;
  total_publishers: number;
  verification_percentage: number;
}
interface AnalyticsSummaryRaw {
  network_usage: { network: string; contract_count: number }[];
  category_distribution: { category: string; contract_count: number }[];
  top_publishers: { publisher_id: string; name: string; contract_count: number }[];
}
interface AnalyticsDashboardRaw {
  deployment_trends: { date: string; count: number }[];
}

/**
 * Builds the stats page model from the backend responses. /api/stats has
 * the totals, /api/analytics/summary the breakdowns and top publishers,
 * and /api/analytics/dashboard the period-scoped deployment trend.
 */
export function toStatsResponse(
  stats: RegistryStatsRaw,
  summary: AnalyticsSummaryRaw,
  dashboard: AnalyticsDashboardRaw,
): StatsResponse {
  return {
    totalContracts: stats.total_contracts,
    verifiedPercentage: stats.verification_percentage,
    totalPublishers: stats.total_publishers,
    networkBreakdown: summary.network_usage.map((n) => ({
      network: n.network,
      contracts: n.contract_count,
    })),
    contractsByCategory: summary.category_distribution.map((c) => ({
      category: c.category,
      count: c.contract_count,
    })),
    deploymentsTrend: dashboard.deployment_trends,
    topPublishers: summary.top_publishers.slice(0, 5).map((p) => ({
      name: p.name,
      // The publisher route takes the publisher id.
      address: p.publisher_id,
      contractsDeployed: p.contract_count,
    })),
  };
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_URL}${path}`);
  if (!res.ok) {
    throw new Error(`Failed to fetch ${path}: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

export async function fetchStats(period: TimePeriod): Promise<StatsResponse> {
  if (!USE_MOCKS) {
    // The dashboard has no all-time range; its longest window is 90 days.
    const timeframe = period === "all-time" ? "90d" : period;
    const [stats, summary, dashboard] = await Promise.all([
      getJson<RegistryStatsRaw>(`/api/stats?period=${encodeURIComponent(period)}`),
      getJson<AnalyticsSummaryRaw>("/api/analytics/summary"),
      getJson<AnalyticsDashboardRaw>(`/api/analytics/dashboard?timeframe=${timeframe}`),
    ]);
    return toStatsResponse(stats, summary, dashboard);
  }

  // ---------------------------------------------------------------------------
  // Mock fallback (development only, gated behind NEXT_PUBLIC_USE_MOCKS)
  // ---------------------------------------------------------------------------
  const delay = getRandomInt(300, 600);
  await new Promise((resolve) => setTimeout(resolve, delay));

  let trendDays = 30;
  if (period === "7d") trendDays = 7;
  if (period === "90d") trendDays = 90;

  const totalContracts = getRandomInt(1000, 5000);
  const verifiedPercentage = getRandomInt(60, 95);
  const totalPublishers = getRandomInt(100, 500);

  const networkBreakdown = NETWORKS.map((network) => ({
    network,
    contracts: getRandomInt(100, totalContracts / 2),
  }));

  const contractsByCategory = CATEGORIES.map((category) => ({
    category,
    count: getRandomInt(10, 500),
  })).sort((a, b) => b.count - a.count);

  const deploymentsTrend = generateTrendData(trendDays);

  const topPublishers = PUBLISHER_NAMES.map((name, index) => ({
    name,
    address: `G${name.toUpperCase().substring(0, 5)}...MOCK${index}`,
    contractsDeployed: getRandomInt(10, 200),
  }))
    .sort((a, b) => b.contractsDeployed - a.contractsDeployed)
    .slice(0, 5);

  return {
    totalContracts,
    verifiedPercentage,
    totalPublishers,
    networkBreakdown,
    contractsByCategory,
    deploymentsTrend,
    topPublishers,
  };
}

// ---------------------------------------------------------------------------
// Mock data helpers (only used when USE_MOCKS === true)
// ---------------------------------------------------------------------------

const CATEGORIES = [
  "DeFi",
  "NFT",
  "Gaming",
  "Infrastructure",
  "DAO",
  "Wallet",
  "Social",
  "Tooling",
];

const NETWORKS = ["Mainnet", "Testnet", "Futurenet"];

const PUBLISHER_NAMES = [
  "StellarFoundation",
  "SorobanLabs",
  "DefiKingdoms",
  "OpenSea",
  "Coinbase",
  "Kraken",
  "Circle",
  "SettleNetwork",
  "UltraStellar",
  "Lobstr",
];

function getRandomInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function generateTrendData(days: number): { date: string; count: number }[] {
  const data = [];
  const today = new Date();
  for (let i = days - 1; i >= 0; i--) {
    const date = new Date(today);
    date.setDate(date.getDate() - i);
    data.push({
      date: date.toISOString().split("T")[0],
      count: getRandomInt(5, 50),
    });
  }
  return data;
}

// ---------------------------------------------------------------------------
// Moved here from lib/api.ts (stage 5b)
// ---------------------------------------------------------------------------

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
