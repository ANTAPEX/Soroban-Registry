/**
 * The query layer: one hook per API endpoint, each owning its cache key.
 *
 * A component that needs data from the registry calls a hook from here rather
 * than pairing `useQuery` with a hand-written key and a `lib/api` import. The
 * modules mirror `lib/api/` one for one.
 *
 * Queries that compose or derive rather than call an endpoint — the version
 * compatibility view, the diff viewer's source fetches, the comparison tray —
 * keep their own `useQuery` at the call site and take their key from
 * `lib/queryKeys.ts`.
 */

export * from "./activity";
export * from "./comments";
export * from "./compatibility";
export * from "./contracts";
export * from "./dependencies";
export * from "./graph";
export * from "./metrics";
export * from "./preferences";
export * from "./publishers";
export * from "./registry";
export * from "./releaseNotes";
export * from "./reviews";
export * from "./search";
export * from "./types";
export * from "./verification";
