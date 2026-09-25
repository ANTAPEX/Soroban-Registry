import { API_URL, USE_MOCKS } from "@/lib/env";
import type {
  PublisherResponse,
  ContractSummary,
  ActivityEvent,
  Contract,
  ContractSearchParams,
  PaginatedResponse,
  Publisher,
} from "@/types";
import { apiFetch } from "./client";
import { ApiError } from "@/lib/errors";
import { MOCK_CONTRACTS } from "./mocks";


// ---------------------------------------------------------------------------
// Real API call (primary path)
// ---------------------------------------------------------------------------

// Raw shapes of the two backend endpoints the publisher page combines.
interface PublisherRecordRaw {
  id: string;
  stellar_address: string;
  username?: string | null;
  website?: string | null;
  github_url?: string | null;
  created_at: string;
}
interface PublisherSummaryRaw {
  publisher: PublisherRecordRaw;
  contract_count: number;
  verified_contract_count: number;
}
interface PublisherContractRaw {
  id: string;
  name: string;
  description?: string | null;
  is_verified: boolean;
  verification_status?: string | null;
  tags?: string[] | null;
  created_at: string;
  deployed_at?: string | null;
  verified_at?: string | null;
  artifact_scan_status?: "pending" | "passed" | "quarantined" | null;
}

function toVerificationStatus(c: PublisherContractRaw): ContractSummary["verificationStatus"] {
  if (c.is_verified) return "verified";
  return c.verification_status === "failed" ? "failed" : "pending";
}

/**
 * Builds the publisher page model from /api/publishers/:id/summary and
 * /api/publishers/:id/contracts. The backend has no activity feed for a
 * publisher, so activity is derived from the contracts: one event per
 * publication, plus one per verification that has a date.
 */
export function toPublisherResponse(
  summary: PublisherSummaryRaw,
  contracts: PublisherContractRaw[],
): PublisherResponse {
  const { publisher } = summary;
  const activity: ActivityEvent[] = contracts
    .flatMap((c) => {
      const events: ActivityEvent[] = [
        { id: `${c.id}-published`, type: "contract_published", contractName: c.name, timestamp: c.created_at },
      ];
      if (c.verified_at) {
        events.push({ id: `${c.id}-verified`, type: "verification_success", contractName: c.name, timestamp: c.verified_at });
      }
      return events;
    })
    .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
    .slice(0, 10);

  return {
    address: publisher.stellar_address,
    displayName: publisher.username || publisher.stellar_address,
    website: publisher.website ?? undefined,
    github: publisher.github_url ?? undefined,
    verifiedContracts: summary.verified_contract_count,
    failedVerifications: contracts.filter((c) => toVerificationStatus(c) === "failed").length,
    totalContracts: summary.contract_count,
    createdAt: publisher.created_at,
    contracts: contracts.map((c) => ({
      id: c.id,
      name: c.name,
      description: c.description ?? "",
      verificationStatus: toVerificationStatus(c),
      deployedAt: c.deployed_at ?? c.created_at,
      tags: c.tags ?? [],
      artifactScanStatus: c.artifact_scan_status ?? "pending",
    })),
    activity,
  };
}

export async function getPublisher(
  address: string,
): Promise<PublisherResponse> {
  if (!USE_MOCKS) {
    const id = encodeURIComponent(address);
    const get = async <T,>(path: string): Promise<T> => {
      const res = await fetch(`${API_URL}${path}`);
      if (!res.ok) {
        // ApiError carries the status, so a 404 is not retried.
        throw new ApiError(`Failed to fetch publisher: ${res.status}`, res.status, undefined, path);
      }
      return res.json() as Promise<T>;
    };
    const [summary, contracts] = await Promise.all([
      get<PublisherSummaryRaw>(`/api/publishers/${id}/summary`),
      get<{ items: PublisherContractRaw[] }>(`/api/publishers/${id}/contracts?limit=100`),
    ]);
    return toPublisherResponse(summary, contracts.items);
  }

  // ---------------------------------------------------------------------------
  // Mock fallback (development only, gated behind NEXT_PUBLIC_USE_MOCKS)
  // ---------------------------------------------------------------------------
  await new Promise((resolve) => setTimeout(resolve, 800));

  const contracts = generateMockContracts(18);
  const activity = generateMockActivity(12);

  return {
    ...MOCK_PUBLISHER,
    address: address,
    avatarUrl: `https://api.dicebear.com/7.x/identicon/svg?seed=${address}`,
    contracts,
    activity,
  };
}

// ---------------------------------------------------------------------------
// Mock data generators (only used when USE_MOCKS === true)
// ---------------------------------------------------------------------------

// Mock data generator helper
const generateMockContracts = (count: number): ContractSummary[] => {
  return Array.from({ length: count }).map((_, i) => {
    const statusRand = Math.random();
    let status: "verified" | "failed" | "pending" = "verified";
    if (statusRand > 0.8) status = "failed";
    else if (statusRand > 0.6) status = "pending";

    return {
      id: `C${Math.random().toString(36).substring(2, 15).toUpperCase()}`,
      name: `Soroban Contract ${i + 1}`,
      description: `A sample Soroban smart contract for demonstration purposes. This contract handles specific logic for the dApp ecosystem.`,
      verificationStatus: status,
      deployedAt: new Date(
        Date.now() - Math.random() * 10000000000,
      ).toISOString(),
      tags: ["defi", "nft", "governance"].filter(() => Math.random() > 0.5),
      artifactScanStatus: statusRand > 0.8 ? "quarantined" : "passed",
    };
  });
};

const generateMockActivity = (count: number): ActivityEvent[] => {
  return Array.from({ length: count }).map(() => {
    const typeRand = Math.random();
    let type:
      | "verification_success"
      | "verification_failed"
      | "contract_published" = "contract_published";
    if (typeRand > 0.6) type = "verification_success";
    else if (typeRand > 0.4) type = "verification_failed";

    return {
      id: `act_${Math.random().toString(36).substring(2, 9)}`,
      type: type,
      contractName: `Soroban Contract ${Math.floor(Math.random() * 10) + 1}`,
      timestamp: new Date(
        Date.now() - Math.random() * 5000000000,
      ).toISOString(),
    };
  });
};

const MOCK_PUBLISHER: Omit<PublisherResponse, "contracts" | "activity"> = {
  address: "GBSX...2J4K", // This will be overwritten by the requested address
  displayName: "Stellar builder",
  bio: "Building decentralized applications on Soroban. Passionate about DeFi and DAO governance structures.",
  avatarUrl: "https://api.dicebear.com/7.x/identicon/svg?seed=stellar",
  website: "https://stellar.org",
  github: "https://github.com/stellar",
  verifiedContracts: 15,
  failedVerifications: 2,
  totalContracts: 18,
  createdAt: "2023-09-15T10:00:00Z",
};

// ---------------------------------------------------------------------------
// Moved here from lib/api.ts (stage 5b)
// ---------------------------------------------------------------------------

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
