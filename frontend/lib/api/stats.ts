import { StatsResponse, TimePeriod } from '@/types/stats';

const CATEGORIES = [
  'DeFi',
  'NFT',
  'Gaming',
  'Infrastructure',
  'DAO',
  'Wallet',
  'Social',
  'Tooling',
];

const NETWORKS = ['Mainnet', 'Testnet', 'Futurenet'];

const PUBLISHER_NAMES = [
  'StellarFoundation',
  'SorobanLabs',
  'DefiKingdoms',
  'OpenSea',
  'Coinbase',
  'Kraken',
  'Circle',
  'SettleNetwork',
  'UltraStellar',
  'Lobstr',
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
      date: date.toISOString().split('T')[0],
      count: getRandomInt(5, 50),
    });
  }
  
  return data;
}

export async function fetchStats(period: TimePeriod): Promise<StatsResponse> {
  // Simulate network delay (300-600ms)
  const delay = getRandomInt(300, 600);
  await new Promise((resolve) => setTimeout(resolve, delay));

  // Determine trend days based on period
  let trendDays = 30; // Default for '30d' and 'all-time' (for trend chart clarity)
  if (period === '7d') trendDays = 7;
  if (period === '90d') trendDays = 90;

  // Generate mock data
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

  const topPublishers = PUBLISHER_NAMES.map((name) => ({
    name,
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
