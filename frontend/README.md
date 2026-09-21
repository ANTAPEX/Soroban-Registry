# Frontend

The Soroban Registry web app: Next.js 15 (App Router), React 19, TypeScript, Tailwind,
Redux Toolkit for UI state and TanStack Query for server state.

Setup, environment variables and the rest of the stack are in the
[repository README](../README.md). This file covers what is specific to `frontend/`.

## Running it

```bash
npm ci --legacy-peer-deps
cp .env.example .env.local     # then set NEXT_PUBLIC_API_URL=http://localhost:3001
npm run dev                    # http://localhost:3000
```

`--legacy-peer-deps` is required, not a workaround for a broken machine: `@reduxjs/toolkit`
1.9 declares a peer of React 18 and this app is on React 19, so a plain `npm ci` fails with
`ERESOLVE`. The committed `package-lock.json` was resolved the same way. There are also
`pnpm-lock.yaml` and `yarn.lock` in this directory; npm is what CI uses.

## How it is laid out

| Path | What lives there |
| --- | --- |
| `app/` | Routes. App Router conventions: `page`, `layout`, `loading`, `error`, `route`. |
| `components/` | React components. Storybook stories sit beside the component they document. |
| `lib/api/` | One module per API domain. `lib/api.ts` is a barrel that re-exports them. |
| `lib/env.ts` | **The only module that reads `process.env`.** Add a variable here, not at its call site. |
| `lib/queryKeys.ts` | Every TanStack Query cache key, in one place. |
| `hooks/queries/` | One hook per endpoint, mirroring `lib/api/`. |
| `types/` | Domain types. This directory imports nothing from the rest of the app. |
| `store/` | Redux Toolkit slices for UI state. |
| `__tests__/` | Jest suites. |

Two conventions the layout depends on, both greppable:

```bash
# Nothing outside lib/env.ts may read process.env
grep -rn "process\.env\." --include='*.ts*' app components hooks lib store providers types | grep -v "^lib/env.ts"

# Nothing may build a query key inline
grep -rn 'queryKey:\s*\[' --include='*.ts*' app components hooks lib
```

Both should return nothing.

## Checks

```bash
npx jest --runInBand     # `npm test` is the same run plus coverage
npx eslint .
npx tsc --noEmit
npx next build
```

**Always run `next build` before pushing.** `tsc` and `eslint` both pass on a mistake the
Vercel build rejects: `"use client"` has to be the first statement in a file, and an import
inserted above it silently demotes the page to a Server Component.

Five suites fail on `main` today and are not caused by your branch:
`__tests__/lib/api.test.ts`, `ipfsMirror`, `contractsContentFilters`,
`components/FilterPanel` and `resilience`. Compare against a clean `main` rather than
expecting green.

## Storybook

```bash
npm run storybook        # http://localhost:6006
```

Stories live next to their components under `components/`, which is what
`.storybook/main.ts` globs.
