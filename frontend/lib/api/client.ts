/**
 * The one HTTP client for the Soroban Registry API.
 *
 * Every call to the backend goes through `apiFetch`, so base URL resolution,
 * the auth header, the CSRF handshake and retry, and error mapping are decided
 * once. Extracted from `lib/api.ts`, where it was private and therefore
 * duplicated — nine other modules had grown their own `fetch` wrappers, each
 * with its own error handling and its own base-URL fallback.
 */

import { API_URL } from "@/lib/env";
import { NetworkError, createApiError, extractErrorData } from "@/lib/errors";

const AUTH_TOKEN_KEY = "soroban_registry_token";

// The backend rejects browser mutation requests (POST/PUT/PATCH/DELETE)
// without a matching x-csrf-token header + sr_csrf cookie pair (see
// security.rs's csrf_and_origin_middleware). Fetch and cache the token once,
// re-fetching if a request comes back CSRF-rejected (e.g. the cookie expired).
const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);
let csrfTokenPromise: Promise<string> | null = null;

async function fetchCsrfToken(): Promise<string> {
  const res = await fetch(`${API_URL}/api/auth/csrf`, { credentials: "include" });
  if (!res.ok) throw new Error(`Failed to fetch CSRF token: ${res.status}`);
  const data = (await res.json()) as { token: string };
  return data.token;
}

function getCsrfToken(): Promise<string> {
  if (!csrfTokenPromise) {
    csrfTokenPromise = fetchCsrfToken().catch((err) => {
      csrfTokenPromise = null;
      throw err;
    });
  }
  return csrfTokenPromise;
}

export async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const url = `${API_URL}${path}`;
  const method = (options?.method || "GET").toUpperCase();
  const isMutating = MUTATING_METHODS.has(method);

  const buildHeaders = (csrfToken?: string) => {
    const headers = new Headers(options?.headers);
    if (!headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }
    if (!headers.has("Authorization") && typeof window !== "undefined") {
      try {
        const token = window.localStorage.getItem(AUTH_TOKEN_KEY);
        if (token) headers.set("Authorization", `Bearer ${token}`);
      } catch {
        // Storage may be unavailable in hardened/private browsing contexts.
      }
    }
    if (csrfToken) headers.set("x-csrf-token", csrfToken);
    return headers;
  };

  const doFetch = async (csrfToken?: string) =>
    fetch(url, {
      credentials: "include",
      ...options,
      headers: buildHeaders(csrfToken),
    });

  let response: Response;
  try {
    response = isMutating ? await doFetch(await getCsrfToken()) : await doFetch();
    // The cached token can go stale (cookie expiry); refresh once and retry.
    if (isMutating && response.status === 403) {
      const { details } = await extractErrorData(response.clone());
      const errorCode = (details as { error_code?: string } | undefined)?.error_code;
      if (errorCode?.startsWith("CSRF")) {
        csrfTokenPromise = null;
        response = await doFetch(await getCsrfToken());
      }
    }
  } catch (err) {
    throw new NetworkError(`Network request failed: ${String(err)}`);
  }
  if (!response.ok) {
    const data = await extractErrorData(response);
    throw createApiError(response.status, data, path);
  }
  return response.json() as Promise<T>;
}
