import { toStatsResponse } from "@/lib/api/stats";

test("toStatsResponse maps the backend responses onto the stats page model", () => {
  const result = toStatsResponse(
    { total_contracts: 50, total_publishers: 10, verification_percentage: 34 },
    {
      network_usage: [{ network: "mainnet", contract_count: 17 }],
      category_distribution: [{ category: "DeFi", contract_count: 7 }],
      top_publishers: Array.from({ length: 7 }, (_, i) => ({
        publisher_id: `id-${i}`,
        name: `pub-${i}`,
        contract_count: 10 - i,
      })),
    },
    { deployment_trends: [{ date: "2026-09-01", count: 3 }] },
  );

  expect(result.totalContracts).toBe(50);
  expect(result.verifiedPercentage).toBe(34);
  expect(result.totalPublishers).toBe(10);
  expect(result.networkBreakdown).toEqual([{ network: "mainnet", contracts: 17 }]);
  expect(result.contractsByCategory).toEqual([{ category: "DeFi", count: 7 }]);
  expect(result.deploymentsTrend).toEqual([{ date: "2026-09-01", count: 3 }]);
  expect(result.topPublishers).toHaveLength(5);
  expect(result.topPublishers[0]).toEqual({ name: "pub-0", address: "id-0", contractsDeployed: 10 });
});
