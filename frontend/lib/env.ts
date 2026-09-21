/**
 * The single place the frontend reads its environment.
 *
 * Next.js inlines `process.env.NEXT_PUBLIC_*` at build time wherever the literal
 * member expression appears, so every variable must be spelled out here rather
 * than looked up dynamically. Everything else in the app imports from this module,
 * which keeps the defaults in one place — before this existed, `NEXT_PUBLIC_API_URL`
 * was read in ten files and fell back to `""` in eight of them and to
 * `"http://localhost:3001"` in the other two.
 *
 * An empty `API_URL` is meaningful: it makes the app use relative `/api/*` paths,
 * which is how a same-domain deployment is configured. See `.env.example`.
 */

/** Backend API origin. Empty string means "use relative paths". */
export const API_URL = (process.env.NEXT_PUBLIC_API_URL || "").replace(/\/$/, "");

/** Serve local mock data instead of calling the API. Development only. */
export const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === "true";

/** Stellar network the live data adapters query. */
export const STELLAR_NETWORK = process.env.NEXT_PUBLIC_STELLAR_NETWORK || "mainnet";

/** Activity feed / recent contracts poll interval, in milliseconds. */
export const ACTIVITY_POLL_MS = Number(
  process.env.NEXT_PUBLIC_ACTIVITY_POLL_MS || 30_000,
);

/** Client-side error reporting. Disabled by setting the variable to "false". */
export const ERROR_REPORTING_ENABLED =
  process.env.NEXT_PUBLIC_ERROR_REPORTING !== "false";

/** Analytics provider: "ga" | "plausible" | "mixpanel". */
export const ANALYTICS_PROVIDER =
  process.env.NEXT_PUBLIC_ANALYTICS_PROVIDER || "ga";

export const GA_ID = process.env.NEXT_PUBLIC_GA_ID;
export const PLAUSIBLE_DOMAIN = process.env.NEXT_PUBLIC_PLAUSIBLE_DOMAIN;
export const MIXPANEL_TOKEN = process.env.NEXT_PUBLIC_MIXPANEL_TOKEN;

export const IS_PRODUCTION = process.env.NODE_ENV === "production";

/**
 * Join the API origin with a path. Works whether or not `API_URL` is set, so
 * callers never have to think about the same-domain case.
 */
export function apiUrl(path: string): string {
  return `${API_URL}${path.startsWith("/") ? path : `/${path}`}`;
}
