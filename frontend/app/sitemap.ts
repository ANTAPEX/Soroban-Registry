import type { MetadataRoute } from "next";

const BASE_URL = "https://soroban-registry.com";

// Static, public content routes only. Excluded on purpose:
// - /dev/* (disallowed in robots.ts, internal debug scaffolding)
// - /favorites and /settings (per-viewer preference pages, no distinct
//   public content, not blocked from indexing — just not worth listing)
// - dynamic routes (/contracts/[id] and its five sub-routes,
//   /publishers/[address]) — listing those needs to enumerate real
//   contract/publisher IDs from the API, which is a separate, larger
//   change from "a sitemap.ts exists at all."
const STATIC_ROUTES: Array<{
  path: string;
  changeFrequency: MetadataRoute.Sitemap[number]["changeFrequency"];
  priority: number;
}> = [
  { path: "/", changeFrequency: "daily", priority: 1 },
  { path: "/contracts", changeFrequency: "daily", priority: 0.9 },
  { path: "/marketplace", changeFrequency: "daily", priority: 0.9 },
  { path: "/templates", changeFrequency: "weekly", priority: 0.8 },
  { path: "/verify-contract", changeFrequency: "weekly", priority: 0.7 },
  { path: "/verification-status", changeFrequency: "weekly", priority: 0.6 },
  { path: "/compare", changeFrequency: "weekly", priority: 0.6 },
  { path: "/graph", changeFrequency: "weekly", priority: 0.6 },
  { path: "/developer", changeFrequency: "monthly", priority: 0.6 },
  { path: "/analytics", changeFrequency: "weekly", priority: 0.5 },
  { path: "/stats", changeFrequency: "weekly", priority: 0.5 },
  { path: "/publish", changeFrequency: "monthly", priority: 0.5 },
  { path: "/contracts/import-export", changeFrequency: "monthly", priority: 0.4 },
];

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date();
  return STATIC_ROUTES.map(({ path, changeFrequency, priority }) => ({
    url: `${BASE_URL}${path}`,
    lastModified,
    changeFrequency,
    priority,
  }));
}
