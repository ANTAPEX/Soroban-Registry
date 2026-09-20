/**
 * Maintenance window endpoint.
 */

import { apiFetch } from "./client";
import type { MaintenanceWindow } from "@/types";

export async function fetchMaintenanceWindow(): Promise<MaintenanceWindow | null> {
  try {
    return await apiFetch<MaintenanceWindow>("/api/maintenance");
  } catch {
    return null;
  }
}
