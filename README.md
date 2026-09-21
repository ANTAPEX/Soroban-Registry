# Soroban Registry

A contract registry and package manager for the Soroban smart-contract ecosystem on
Stellar. It lets developers publish, discover and verify Soroban contracts across Stellar
networks, the way npm and crates.io serve the JavaScript and Rust communities.

Production: <https://soroban-registry.vercel.app/>

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-nightly%20(pinned)-orange.svg)
![Next.js](https://img.shields.io/badge/next.js-15-black.svg)

---

## Get it running

Two long-running processes and about ten minutes, most of which is the first Rust build.
Every command below was run against a checkout of `main` before it was written down.

### 0. What you need

| Tool | Version | How it is pinned |
| --- | --- | --- |
| Rust | nightly | `rust-toolchain.toml` at the repo root. rustup installs it on your first `cargo` command; do not override it with `+stable`. |
| Node.js | `^18.18` or `^19.8` or `>=20` | Next.js 15.3's own `engines` field. |
| PostgreSQL | 16 | What `docker-compose.yml` runs and what the migrations are tested against. |
| Docker | optional | Only for the observability stack. The app itself does not need it. |

You do **not** need the SQLx CLI, and you do **not** need a database to compile anything.
The backend has no `sqlx::query!` macros left, so `cargo build` works on a machine with no
Postgres at all.

### 1. Database

```bash
createdb soroban_registry
```

That is the whole step. The API applies all 143 migrations in `database/migrations/` itself
on startup, in order, and records them in `_sqlx_migrations`. Set `SKIP_MIGRATIONS=true` if
you would rather apply them by hand.

If you want to apply them without starting the API:

```bash
cargo install sqlx-cli --version 0.8.6 --no-default-features --features rustls,postgres --locked
DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry" \
  sqlx migrate run --source database/migrations
```

### 2. Backend API

```bash
cp .env.example .env
```

Then add these three lines to `.env`. **`.env.example` is missing them and the API will not
start without them** — see [The environment the API requires](#the-environment-the-api-requires).

```bash
ELASTICSEARCH_URL=http://localhost:9200
HOST=0.0.0.0
LOG_LEVEL=info
```

```bash
cd backend
cargo run --bin api
```

The first build takes a few minutes. You are up when the log says:

```
{"level":"INFO","fields":{"message":"API server listening on 0.0.0.0:3001"},"target":"api"}
```

```bash
curl http://localhost:3001/health
# {"status":"healthy","timestamp":"...","version":"0.1.0"}
```

Elasticsearch and Redis do not have to be running. `ELASTICSEARCH_URL` has to be *set*, but
nothing connects to it at startup, and Redis caching is off unless `REDIS_ENABLED=true`.

### 3. Seed some data

A fresh registry is empty, and an empty registry makes the UI hard to judge.

```bash
cd backend
cargo run --bin seeder -- --count 50
```

That writes publishers, contracts, versions and verifications, and takes under a second.
`--seed <n>` makes it reproducible; `--data-file <path>` loads your own fixtures.

### 4. Frontend

```bash
cd frontend
npm ci --legacy-peer-deps
cp .env.example .env.local     # then set NEXT_PUBLIC_API_URL=http://localhost:3001
npm run dev
```

`--legacy-peer-deps` is not optional. `@reduxjs/toolkit` 1.9 declares a peer of React 18 and
this app is on React 19, so a plain `npm ci` fails with `ERESOLVE`. The committed lockfile
was resolved the same way.

Open <http://localhost:3000>.

### 5. CLI (optional)

```bash
cargo install --path cli        # installs a `soroban-registry` binary
soroban-registry list           # talks to http://localhost:3001 by default
```

---

## Repository map

| Path | What it is |
| --- | --- |
| `backend/` | Rust workspace, seven crates. `api` (the HTTP server, also a library), `indexer`, `seeder`, `verifier`, `shared`, `contract_abi`, `registry_client`. |
| `frontend/` | The Next.js 15 App Router web app. `lib/api/` is one module per API domain, `hooks/queries/` one hook per endpoint, `types/` the domain types. |
| `cli/` | The `soroban-registry` command-line client. See `cli/README.md` — it is a **standalone Cargo package**, not a workspace member, so `cargo` from the repo root will not find it. |
| `database/migrations/` | 143 forward-only `.sql` files. No down-migrations. |
| `docs/` | Operational runbooks: backups, HA, encryption, alerting, pagination. |
| `scripts/` | Backup, restore and disaster-recovery drill scripts. |
| `observability/` | Prometheus, Grafana, Loki and Alertmanager config for the Docker stack. |
| `tagging-service/` | A small Node service that Docker Compose runs on port 3002. |
| `soroban-registry/` | **A different product that happens to live here.** A Soroban *linter*, in its own Cargo workspace with its own README. `backend/api` depends on exactly one of its six crates (`soroban-batch`); nothing builds the other five. Do not confuse it with the repository root. |

### Ports

| Port | Service |
| --- | --- |
| 3000 | Frontend |
| 3001 | Backend API |
| 3002 | Tagging service |
| 3003 | Grafana |
| 5432 | `pgpool` (the connection you should use) |
| 5433 / 5434 | Postgres primary / replica |
| 6379 | Redis |
| 9090 / 9093 | Prometheus / Alertmanager |
| 16686 | Jaeger UI |

---

## The environment the API requires

`backend/api/src/config.rs` deserializes the process environment with `envy`. Six fields have
no default, so the API exits before it does anything if any one of them is missing:

| Variable | Notes |
| --- | --- |
| `DATABASE_URL` | Must start with `postgres://` or `postgresql://`. |
| `JWT_SECRET` | Must be at least 32 characters. |
| `PORT` | The listener actually binds `0.0.0.0:$PORT`. |
| `HOST` | Required, but nothing reads it. |
| `LOG_LEVEL` | Required. `RUST_LOG` is what actually controls tracing. |
| `ELASTICSEARCH_URL` | Required, but it is re-read with a default at the point of use, so the value barely matters. |

`.env.example` supplies the first three and **not** the last three, so copying it verbatim
and running the API fails with:

```
Error: Failed to load configuration from environment variables

Caused by:
    missing value for field elasticsearch_url
```

then `field host`, then `field log_level`, one per run. Adding the three lines from
[step 2](#2-backend-api) is the whole fix.

Everything else is optional and defaulted: `REDIS_URL`, `REDIS_ENABLED`, `SKIP_MIGRATIONS`,
`FEATURE_FLAGS_JSON`, `ENCRYPTION_KEYS`, the `DB_*` pool-tuning variables and the `*_CACHE_TTL`
family. The full annotated list is `.env.example`.

The frontend has its own `frontend/.env.example`, meant to be copied to `.env.local`.
`frontend/lib/env.ts` is the only module that reads `process.env`, so that file is the list of
everything the browser bundle can see. An empty `NEXT_PUBLIC_API_URL` is meaningful: it makes
the app use relative `/api/*` paths, which is how the same-domain production deploy works.

---

## The API surface

The registry serves 391 routes. They are assembled by `application_routes()` in
`backend/api/src/routes.rs`, which merges 47 per-domain route groups; that function is the
table of contents.

A few to start from:

| Route | |
| --- | --- |
| `GET /health` | Liveness. Also `/health/live`, `/health/ready`, `/health/detailed`. |
| `GET /metrics` | Prometheus exposition. |
| `GET /api/contracts` | List and search. |
| `GET /api/contracts/:id` | One contract. 133 routes hang off this prefix. |
| `GET /api/contracts/:id/versions` | Version history. |
| `GET /api/contracts/:id/changelog` | Changelog with breaking-change markers. |
| `GET /api/publishers/:id/contracts` | A publisher's contracts. |
| `GET /api/stats` | Registry-level statistics. |

There is a Swagger UI, but it is behind a Cargo feature that is off by default:

```bash
cargo run --bin api --features openapi
# then open http://localhost:3001/docs
```

---

## Tests and checks

### Frontend

```bash
cd frontend
npx jest --runInBand          # `npm test` adds --coverage
npx next build                # run this before every push, see below
npx eslint .
npx tsc --noEmit
```

**Run `next build` before pushing any frontend change.** `tsc` and `eslint` both miss a class
of error that the Vercel build catches — most memorably that `"use client"` must be the first
statement in a file, so an import inserted above it silently turns a page into a Server
Component.

The suite passes on `main`: 16 suites, 65 tests. CI runs it on every pull request that
touches `frontend/`, so a red suite is your branch's and not the repository's.

### CLI

`cli/README.md` has the full story. The sequence CI runs, which passes on a clean checkout
with no database and no API:

```bash
cargo build --manifest-path cli/Cargo.toml --locked --all-targets
cargo test  --manifest-path cli/Cargo.toml --locked
cargo run   --manifest-path cli/Cargo.toml --locked --bin soroban-registry -- --help
cargo fmt   --manifest-path cli/Cargo.toml -p soroban-registry-cli --check
```

Format with `-p soroban-registry-cli`, never `--all`: the CLI depends on three backend crates
by path, and `--all` follows those paths into the backend's own formatting drift.

### Backend

```bash
cd backend
cargo build                            # no database needed
cargo test --workspace --locked        # 1094 pass, 111 ignored
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

The suite passes on `main` and CI runs it on every pull request that touches Rust. Note that
`cargo build` alone does not compile test targets, so it can succeed while `cargo test` fails;
`cargo check --all-targets` is the quick way to catch that.

The 111 ignored tests each carry a reason in their `#[ignore = "..."]` attribute. Most are
gated on a live API and a Postgres instance, with setup instructions in their file headers.
Run them with `-- --ignored` once you have both.

Builds here are disk-hungry. If you are tight on space:

```bash
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=line-tables-only
export CARGO_PROFILE_TEST_DEBUG=line-tables-only
```

### What a pull request actually runs

Every pull request gets the Vercel build of `frontend/`, the contract crates
(`smart-contract-ci.yml`) and an emoji check. The rest are conditional on paths, and between
them they are what a green check mark now means:

| Job | Runs when the PR touches |
| --- | --- |
| `Backend (build, test)` | `backend/**/*.rs`, either `Cargo.toml`/`Cargo.lock`, `soroban-registry/crates/soroban-batch/`, `rust-toolchain.toml` or `ci.yml`. Runs `cargo test --workspace --locked`. |
| `Frontend (typecheck, lint, test, build)` | `frontend/**` or `ci.yml`. Runs `tsc`, `eslint`, `jest` and a production build. |
| `CLI (fmt, build, test)` | `cli/` or the backend crates it path-depends on. |
| `Check Migration File Naming` | `database/migrations/**`. Runs `.github/scripts/validate-migrations.sh`, which checks both the naming convention and the ordering. |
| `Dependency security audit` | any lockfile or manifest. Also runs weekly on a schedule, because a new advisory against an unchanged lockfile is the case a push trigger cannot catch. Reports, never blocks. |

`soroban-registry/` holds a second Cargo workspace of six crates, of which only
`crates/soroban-batch` is in the backend's build graph. That is why the backend filter names
that one crate rather than the directory.

Nothing deploys on a merge. `deploy.yml` is `workflow_dispatch` only and its deployment steps
are unimplemented placeholders.

Neither the backend nor the frontend job gates on clippy, rustfmt or eslint *warnings*. The
repository has pre-existing drift in all three, so turning them on would make the job red on
arrival. Errors do fail the build.

The emoji check scans whole files rather than changed lines, so touching a file pulls every
pre-existing glyph in it into scope.

---

## Docker Compose

```bash
docker compose up -d postgres-primary postgres-replica pgpool redis
```

That gives you the replicated Postgres pair behind `pgpool` on 5432 plus Redis, which is all
the local stack needs; point `DATABASE_URL` at `localhost:5432` and run the API from source as
above. Named volumes mean the data survives `docker compose down`; only `down -v` deletes it.
The operator runbook is [docs/database-high-availability.md](docs/database-high-availability.md).

Adding `jaeger prometheus grafana loki alertmanager` brings up the observability stack.

**The `api` service in `docker-compose.yml` does not currently start.** Its `environment:`
block sets neither `JWT_SECRET`, `PORT`, `HOST`, `LOG_LEVEL` nor `ELASTICSEARCH_URL`, and there
is no `env_file:`, so the config load described above fails inside the container. Run the API
from source until that is fixed.

---

## Using the CLI

```bash
cargo install --path cli
```

The package is `soroban-registry-cli`; the binary it installs is `soroban-registry`. There are
66 top-level commands — `soroban-registry --help` is the real reference. Some starting points:

```bash
soroban-registry list
soroban-registry search token --verified-only --networks testnet,futurenet --category DeFi
soroban-registry info <contract-id>
soroban-registry contract stats --network testnet
soroban-registry contract export contracts.jsonl --format jsonl --network testnet
soroban-registry dashboard          # interactive terminal dashboard
soroban-registry wizard             # interactive setup
```

It talks to `http://localhost:3001` unless told otherwise. Override with `--api-url`, with
`SOROBAN_REGISTRY_API_URL`, or persistently with `soroban-registry config set`. Settings live
in `~/.soroban-registry/` (`config.toml` and `config.json`); a legacy
`~/.soroban-registry.toml` is migrated automatically.

---

## Troubleshooting

| What you see | What it means |
| --- | --- |
| `missing value for field elasticsearch_url` (or `host`, or `log_level`) | `.env.example` does not define them. Add the three lines from step 2. |
| `JWT_SECRET must be at least 32 characters long` | The placeholder in `.env.example` is long enough; a shorter one of your own is not. |
| `npm ci` fails with `ERESOLVE` on `@reduxjs/toolkit` | Use `npm ci --legacy-peer-deps`. |
| Vercel build red while `tsc` and `eslint` are clean | Almost always a `"use client"` directive that is no longer the first statement. Run `npx next build`. |
| `error: no bin target named 'verifier'` | `verifier` is a library. The three binaries are `api`, `indexer` and `seeder`. |
| `cargo` cannot find the CLI crate from the repo root | `cli/` is not in any workspace. `cd cli`, or pass `--manifest-path cli/Cargo.toml`. |
| The UI renders but every list is empty | Run the seeder. |
| Emoji check fails on a line you never touched | It scans whole files. Remove the pre-existing glyph or leave the file alone. |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for branch naming, commit format and the PR checklist,
and [CREATE_PR_INSTRUCTIONS.md](CREATE_PR_INSTRUCTIONS.md) for the PR mechanics.

Issues and feature requests: <https://github.com/ALIPHATICHYD/Soroban-Registry/issues>

## Community

- Soroban SDK — <https://github.com/stellar/rs-soroban-sdk>
- Stellar docs — <https://developers.stellar.org/>
- Stellar Discord — <https://discord.gg/stellar>

## License

MIT. Every crate manifest in the repository declares `license = "MIT"`.
