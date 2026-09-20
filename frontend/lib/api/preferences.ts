/**
 * User preference endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { UserPreferences } from "@/types";

export async function fetchPreferences(token: string): Promise<UserPreferences> {
  if (USE_MOCKS) {
    return { favorites: [] };
  }
  return apiFetch<UserPreferences>("/api/me/preferences", {
    headers: { Authorization: `Bearer ${token}` },
  });
}

export async function updatePreferences(
  token: string,
  favorites: string[],
): Promise<UserPreferences> {
  if (USE_MOCKS) {
    return { favorites };
  }
  return apiFetch<UserPreferences>("/api/me/preferences", {
    method: "PATCH",
    headers: { Authorization: `Bearer ${token}` },
    body: JSON.stringify({ favorites }),
  });
}
