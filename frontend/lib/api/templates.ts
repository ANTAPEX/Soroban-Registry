/**
 * Contract template endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type { Template } from "@/types";

export async function fetchTemplates(): Promise<Template[]> {
  if (USE_MOCKS) return Promise.resolve([]);
  return apiFetch<Template[]>("/api/templates");
}
