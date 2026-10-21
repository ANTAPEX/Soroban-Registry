---
name: production-readiness
description: This repository's production-readiness rules - which distributed-systems patterns are already implemented and where, which are deliberately absent, and which look done but are thin. Read before adding resilience, caching, queueing, scaling, deployment or observability machinery to the backend, and before concluding that any of them is missing.
---

# Production readiness in this repository

The registry is **one Rust binary (`backend/api`), one Postgres, one indexer,
and a Next.js frontend on Vercel**. Every rule below follows from that shape.
When a general checklist disagrees with this file, this file wins, because the
checklist does not know what is already here.

## Before adding anything, check whether it exists

Most resilience machinery is already implemented. Adding a second copy is the
most common failure mode in this codebase. Verified locations:

| Concern | Already at |
| --- | --- |
| Circuit breaking | `backend/api/src/db_resilience.rs` — three-state FSM, wired in `main.rs`, `handlers.rs`, `auth_handlers.rs` |
| Retries, backoff | `db_resilience.rs`, `backend/indexer/src/backoff.rs` |
| Backpressure | `DbQueue` in `db_resilience.rs` |
| Rate limiting | `rate_limit.rs` |
| Caching, invalidation, idempotency keys | `cache.rs` |
| Connection pooling | `db_pool.rs` |
| Partitioning | `partition_manager.rs` |
| Distributed locks | `pg_advisory` in `migration_handlers.rs` |
| Feature flags | `feature_flags.rs` |
| Health, liveness, readiness | `health.rs`, `health_monitor.rs` |
| Metrics, SLOs, error budgets | `metrics.rs` (`slo_error_budget_remaining`, `slo_burn_rate` at line 328) |
| Tracing | `request_tracing.rs` |
| Alerting | `alerting.rs` |
| Secrets, key rotation, encryption | `crypto/key_manager.rs`, `crypto/service.rs` |
| Backups, DR | `backup_handlers.rs`, `disaster_recovery_models.rs`, pgBackRest |
| Incidents, postmortems | `incident_handlers.rs`, `post_incident_handlers.rs` |

**Reuse these. Do not write a second circuit breaker, backoff helper or cache
layer.**

## Deliberately absent — do not add without being asked

- **Kubernetes, Helm, Terraform, service discovery, leader election, sharding,
  multi-region.** All assume a fleet. One binary and one database do not need
  them, and each adds a failure domain the project does not currently have.
- **A message broker** (Kafka, RabbitMQ, SQS). Queueing runs on Postgres,
  dead-letter handling included. A broker is another thing to run and fail over.
- **gRPC.** No internal service mesh to justify it.
- **Server-Sent Events.** WebSockets (`websocket.rs`) already cover live updates.
  Two push transports is worse than one.
- **Load balancers, reverse proxies, API gateways, WAF, DDoS protection.**
  Platform concerns. They belong in front of the service, not in this repository.

## Known thin spots — real work, if you are looking for some

These are implemented but narrower than they appear. Verified 2026-10-15 at
`c241a08a`.

1. **Idempotency covers one endpoint.** `cache.rs` has the full
   claim/complete/release helper; it is called only from `publish_contract`
   (`handlers.rs:4602`). `routes.rs` registers 153 write endpoints. Any new
   write endpoint should take an `Idempotency-Key`, and existing ones should be
   retrofitted — the helper exists, so this is wiring.
2. **Webhook URLs are not checked for private IPs.** `webhook_delivery.rs:213`
   POSTs to a user-registered URL; registration
   (`subscription_handlers.rs:682`) validates the scheme only, and allows
   `http://localhost`. Link-local and RFC1918 targets pass. Validate the
   resolved host at registration *and* at delivery.
3. **The replica is monitored but never read from.** `db_monitoring.rs` opens a
   pool against `replica_url` and watches lag; `db_pool.rs` has no read/write
   split, so every query goes to the primary.
4. **Almost nothing sets `Cache-Control`.** One header in the whole API
   (`handlers.rs:5456`). Published contract versions are immutable and should
   say so.
5. **The HA tests assert that documentation exists.** All four tests in
   `backend/api/tests/database_replication_ha_tests.rs` check for files and
   config blocks. None performs a failover.

## Choices already made — do not re-litigate per feature

Several checklist items are either/or. This repository has picked:

- **Locking:** both are in use, deliberately. `FOR UPDATE` for governance
  (`governance_handlers.rs`), advisory locks for migrations. Match the
  surrounding code rather than introducing a third approach.
- **Scaling:** vertical, plus Postgres read capacity. Not horizontal.
- **Deploys:** canary (`canary_handlers.rs`), not blue-green or rolling.
- **Live updates:** WebSockets.
- **API versioning:** path-based, `/api/v1`.

## When a checklist item is a failure mode

N+1 queries, race conditions, deadlocks, memory leaks, clock skew, network
partitions, dependency hell and production incidents are not features to add.
They are caught by the machinery in the first table. If one of them is the
concern, reach for `query_monitor.rs`, `query_analysis.rs`, the metrics, or the
tracing — not for new infrastructure.

CAP theorem and eventual consistency are trade-offs, not tasks. This system is
a single Postgres primary: it is CP, and strongly consistent. Say so rather
than adding machinery for a distributed-consensus problem the project does not
have.
