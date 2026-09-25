import { toPublisherResponse } from "@/lib/api/publishers";

const summary = {
  publisher: {
    id: "p1",
    stellar_address: "GABC",
    username: "stellar_labs",
    website: "https://example.com",
    github_url: null,
    created_at: "2026-01-01T00:00:00Z",
  },
  contract_count: 2,
  verified_contract_count: 1,
};

const contracts = [
  { id: "c1", name: "Swap", description: null, is_verified: true, verification_status: "verified", tags: ["defi"], created_at: "2026-02-01T00:00:00Z", verified_at: "2026-02-03T00:00:00Z", artifact_scan_status: "passed" as const },
  { id: "c2", name: "Vault", is_verified: false, verification_status: "failed", tags: null, created_at: "2026-02-02T00:00:00Z", deployed_at: "2026-02-05T00:00:00Z" },
];

test("toPublisherResponse maps the summary and contracts onto the page model", () => {
  const result = toPublisherResponse(summary, contracts);

  expect(result.address).toBe("GABC");
  expect(result.displayName).toBe("stellar_labs");
  expect(result.github).toBeUndefined();
  expect(result.totalContracts).toBe(2);
  expect(result.verifiedContracts).toBe(1);
  expect(result.failedVerifications).toBe(1);
  expect(result.contracts[0]).toMatchObject({ verificationStatus: "verified", description: "", artifactScanStatus: "passed" });
  expect(result.contracts[1]).toMatchObject({ verificationStatus: "failed", tags: [], deployedAt: "2026-02-05T00:00:00Z", artifactScanStatus: "pending" });
});

test("toPublisherResponse derives newest-first activity from contract dates", () => {
  const result = toPublisherResponse(summary, contracts);
  expect(result.activity.map((e) => e.id)).toEqual(["c1-verified", "c2-published", "c1-published"]);
});

test("toPublisherResponse falls back to the address when there is no username", () => {
  const result = toPublisherResponse({ ...summary, publisher: { ...summary.publisher, username: null } }, []);
  expect(result.displayName).toBe("GABC");
});
