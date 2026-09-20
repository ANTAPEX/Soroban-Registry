/**
 * Relevance scoring for the mock-mode search path.
 *
 * Only reached when NEXT_PUBLIC_USE_MOCKS is set: the real backend ranks
 * server-side. Shared by `contracts.fetchContracts` and `search.semanticSearch`.
 */

import type {
  Contract,
  ContractSearchParams,
  Network,
  SearchIntent,
  SearchIntentType,
} from "@/types";

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

export function tokenizeQuery(query: string): string[] {
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

export function detectIntent(query: string, params?: ContractSearchParams): SearchIntent {
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

export function semanticScore(contract: Contract, queryTokens: string[], intent: SearchIntent): number {
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
