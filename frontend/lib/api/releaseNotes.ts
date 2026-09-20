/**
 * Release notes endpoints.
 */

import { apiFetch } from "./client";
import { USE_MOCKS } from "@/lib/env";
import type {
  GenerateReleaseNotesRequest,
  PublishReleaseNotesRequest,
  ReleaseNotesResponse,
  UpdateReleaseNotesRequest,
} from "@/types";

export async function generateReleaseNotes(
  contractId: string,
  data: GenerateReleaseNotesRequest,
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(`/api/contracts/${contractId}/release-notes/generate`, {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function listReleaseNotes(contractId: string): Promise<ReleaseNotesResponse[]> {
  if (USE_MOCKS) {
    return [];
  }
  return apiFetch<ReleaseNotesResponse[]>(`/api/contracts/${contractId}/release-notes`);
}

export async function fetchReleaseNotes(
  contractId: string,
  version: string,
): Promise<ReleaseNotesResponse> {
  if (USE_MOCKS) {
    return {
      id: `${contractId}-${version}`,
      contract_id: contractId,
      version,
      diff_summary: {
        files_changed: 0,
        lines_added: 0,
        lines_removed: 0,
        function_changes: [],
        has_breaking_changes: false,
        features_count: 0,
        fixes_count: 0,
        breaking_count: 0,
      },
      notes_text: "",
      status: "draft",
      generated_by: "mock",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }
  return apiFetch<ReleaseNotesResponse>(`/api/contracts/${contractId}/release-notes/${version}`);
}

export async function updateReleaseNotes(
  contractId: string,
  version: string,
  data: UpdateReleaseNotesRequest,
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(`/api/contracts/${contractId}/release-notes/${version}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
}

export async function publishReleaseNotes(
  contractId: string,
  version: string,
  data: PublishReleaseNotesRequest = {},
): Promise<ReleaseNotesResponse> {
  return apiFetch<ReleaseNotesResponse>(
    `/api/contracts/${contractId}/release-notes/${version}/publish`,
    { method: "POST", body: JSON.stringify(data) },
  );
}
