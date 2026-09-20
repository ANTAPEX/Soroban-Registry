/**
 * Mock fixtures, loaded once at module init and only when mocks are enabled.
 *
 * The dynamic require keeps `lib/mock-data` out of production bundles: with
 * NEXT_PUBLIC_USE_MOCKS unset the branch is statically false and Next.js drops
 * it, so the three exports stay empty and every caller's `if (USE_MOCKS)` guard
 * is dead code.
 */

import { USE_MOCKS } from "@/lib/env";

/* eslint-disable @typescript-eslint/no-explicit-any */
let contracts: any[] = [];
let examples: Record<string, any[]> = {};
let versions: Record<string, any[]> = {};
/* eslint-enable @typescript-eslint/no-explicit-any */

if (USE_MOCKS) {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const mocks = require("../mock-data");
  contracts = mocks.MOCK_CONTRACTS;
  examples = mocks.MOCK_EXAMPLES;
  versions = mocks.MOCK_VERSIONS;
}

export const MOCK_CONTRACTS = contracts;
export const MOCK_EXAMPLES = examples;
export const MOCK_VERSIONS = versions;
