# Observability and Monitoring Setup

## Overview

The Soroban Registry implements comprehensive observability with metrics, logs, and distributed tracing. This document covers monitoring setup, key metrics, logging configuration, and health check endpoints.

## Architecture

```
┌─────────────┐
│ Application │──── Metrics ────▶ Prometheus ────▶ Grafana
│   Services  │                                       │
│  (API, etc) │──── Traces ─────▶ Jaeger        Dashboards
│             │                                       │
│             │──── Logs ───────▶ Loki/Promtail     Alerts
└─────────────┘                      │               │
                                     │               │
                                     ▼               ▼
                              Log Aggregation   AlertManager
                                 (ELK Stack)      │
                                                  │
                                                  ▼
                                        Slack / PagerDuty
```

## Metrics (Prometheus)

### Exposed Metrics Endpoint

All services expose Prometheus metrics at:

```
GET /metrics
Content-Type: text/plain; version=0.0.4
```

### Configuration

**Prometheus Scrape Config** (`observability/prometheus/prometheus.yml`):

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    app: soroban-registry

scrape_configs:
  - job_name: "soroban-registry-api"
    metrics_path: "/metrics"
    scrape_interval: 10s
    static_configs:
      - targets: ["api:3001"]
        labels:
          service: "soroban-registry"
          environment: "production"

  - job_name: "prometheus"
    static_configs:
      - targets: ["localhost:9090"]

  - job_name: "node-exporter"
    static_configs:
      - targets: ["node-exporter:9100"]
```

### Key Metrics to Monitor

#### 1. HTTP Request Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_http_requests_total` | Counter | Total HTTP requests | `method`, `path`, `status` |
| `soroban_http_request_duration_seconds` | Histogram | Request latency distribution | `method`, `path` |
| `soroban_http_requests_in_flight` | Gauge | Current active requests | - |

**Example Queries:**

```promql
# Request rate (per second)
rate(soroban_http_requests_total[5m])

# P99 latency
histogram_quantile(0.99, sum(rate(soroban_http_request_duration_seconds_bucket[5m])) by (le))

# Error rate (5xx responses)
rate(soroban_http_requests_total{status=~"5.."}[5m]) / rate(soroban_http_requests_total[5m])
```

#### 2. Verification Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_verification_latency_seconds` | Histogram | Verification operation latency | `result` |
| `soroban_verification_queue_depth` | Gauge | Pending verifications | - |
| `soroban_verification_total` | Counter | Total verification attempts | `result` |
| `soroban_verification_failures_total` | Counter | Failed verifications | `reason` |

**Example Queries:**

```promql
# Verification queue backlog
soroban_verification_queue_depth

# Verification success rate
rate(soroban_verification_total{result="success"}[10m]) / rate(soroban_verification_total[10m])

# P99 verification latency
histogram_quantile(0.99, sum(rate(soroban_verification_latency_seconds_bucket[5m])) by (le))
```

#### 3. Database Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_db_connections_active` | Gauge | Active connections | - |
| `soroban_db_pool_size` | Gauge | Connection pool size | - |
| `soroban_db_query_duration_seconds` | Histogram | Query execution time | `operation` |
| `soroban_db_queries_total` | Counter | Total queries | `operation`, `result` |

**Example Queries:**

```promql
# Connection pool utilization
soroban_db_connections_active / soroban_db_pool_size

# Slow query rate (>100ms)
rate(soroban_db_query_duration_seconds_count{quantile="0.99"}[5m])

# Query error rate
rate(soroban_db_queries_total{result="error"}[5m]) / rate(soroban_db_queries_total[5m])
```

#### 4. Cache Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_cache_hits_total` | Counter | Cache hits | `cache_name` |
| `soroban_cache_misses_total` | Counter | Cache misses | `cache_name` |
| `soroban_cache_size_bytes` | Gauge | Cache memory usage | `cache_name` |
| `soroban_cache_evictions_total` | Counter | Cache evictions | `cache_name` |

**Example Queries:**

```promql
# Cache hit ratio
rate(soroban_cache_hits_total[10m]) /
  (rate(soroban_cache_hits_total[10m]) + rate(soroban_cache_misses_total[10m]))

# Cache memory usage (MB)
soroban_cache_size_bytes / 1024 / 1024
```

#### 5. Rate Limiting Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_rate_limit_exceeded_total` | Counter | Rate limit violations | `ip`, `endpoint` |
| `soroban_rate_limit_current_usage` | Gauge | Current rate limit usage | `ip`, `endpoint` |

**Example Queries:**

```promql
# Rate limit violations per minute
rate(soroban_rate_limit_exceeded_total[1m]) * 60

# Top rate-limited IPs
topk(10, sum by (ip) (rate(soroban_rate_limit_exceeded_total[5m])))
```

#### 6. Indexer Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_indexer_last_ledger` | Gauge | Last processed ledger | - |
| `soroban_indexer_lag_seconds` | Gauge | Indexer lag behind chain | - |
| `soroban_indexer_errors_total` | Counter | Indexer errors | `type` |
| `soroban_indexer_rpc_latency_seconds` | Histogram | RPC call latency | `method` |

**Example Queries:**

```promql
# Indexer lag
soroban_indexer_lag_seconds

# RPC error rate
rate(soroban_indexer_errors_total{type="rpc_error"}[5m])
```

#### 7. SLO / SLI Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `soroban_slo_burn_rate` | Gauge | Error budget burn rate | `slo` |
| `soroban_sli_availability` | Gauge | Service availability (0-1) | - |
| `soroban_sli_latency_p99` | Gauge | P99 latency in seconds | - |

**Example Queries:**

```promql
# Availability SLI (99.9% target)
soroban_sli_availability > 0.999

# Error budget remaining
1 - soroban_slo_burn_rate{slo="availability"}
```

## Error Logging

Structured error reporting is available for both API and frontend failures.

- Backend `ApiError` responses emit structured logs with request/correlation IDs.
- Frontend error boundaries normalize errors, redact common secrets, and POST reports to `POST /api/errors/report`.
- Error reports are stored in `error_logs` with indexes for `created_at`, `severity`, `category`, and `request_id`.
- `GET /api/errors/dashboard` returns a 24-hour summary by severity/category plus recent critical errors.
- Critical reports emit an `alert=true` structured log field so log pipelines can route them to Alertmanager, Slack, or PagerDuty.

Sensitive keys containing `password`, `secret`, `token`, `api_key`, `authorization`, or `cookie` are replaced with `[REDACTED]` before persistence.

## Logging

### Structured JSON Logging

All services emit structured JSON logs for easy parsing and aggregation:

```json
{
  "timestamp": "2026-02-24T12:34:56.789Z",
  "level": "INFO",
  "message": "Contract verification completed",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "contract_id": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "duration_ms": 245,
  "result": "success",
  "service": "verifier",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7"
}
```

### Log Levels

| Level | Usage | Examples |
|-------|-------|----------|
| `ERROR` | Operational failures requiring attention | DB connection failures, verification errors |
| `WARN` | Degraded performance, recoverable issues | High latency, retry attempts, deprecated API usage |
| `INFO` | Normal operational events | Request completion, verification success |
| `DEBUG` | Detailed diagnostic information | SQL queries, cache lookups, state transitions |
| `TRACE` | Very detailed debugging | Request/response payloads, internal state |

### Log Aggregation with Loki

**Promtail Configuration** (`observability/loki/promtail-config.yml`):

```yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: soroban-registry
    static_configs:
      - targets:
          - localhost
        labels:
          job: soroban-api
          __path__: /var/log/soroban/*.log
    pipeline_stages:
      - json:
          expressions:
            level: level
            timestamp: timestamp
            correlation_id: correlation_id
            service: service
      - labels:
          level:
          service:
      - timestamp:
          source: timestamp
          format: RFC3339Nano
```

**LogQL Queries:**

```logql
# All errors in last hour
{service="soroban-api"} |= "ERROR" | json

# Slow requests (>500ms)
{service="soroban-api"} | json | duration_ms > 500

# Errors for specific contract
{service="verifier"} | json | contract_id="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC" |= "ERROR"

# Rate limit violations
{service="soroban-api"} | json | message=~".*rate limit.*"
```

### ELK Stack Integration (Alternative)

For environments using Elasticsearch/Logstash/Kibana:

**Filebeat Configuration:**

```yaml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/soroban/*.log
    json.keys_under_root: true
    json.add_error_key: true
    fields:
      service: soroban-registry
      environment: production

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "soroban-registry-%{+yyyy.MM.dd}"

processors:
  - add_host_metadata: ~
  - add_cloud_metadata: ~
```

**Kibana Queries:**

```
# Error rate spike
level: ERROR AND @timestamp: [now-5m TO now]

# Verification failures
service: verifier AND result: failure

# Slow queries
duration_ms: >500 AND path: /api/contracts/search
```

## Distributed Tracing (Jaeger)

### Overview

Distributed traces are exported via OTLP to Jaeger from backend services:
- `soroban-registry-api`
- `soroban-registry-indexer`

Verification flows are instrumented under the verifier component spans so they are visible in request traces.

### Trace Context Propagation

Traces use W3C Trace Context headers:

```http
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
tracestate: vendor=value
```

### Viewing Traces

Access Jaeger UI at: `http://localhost:16686`

**Common Queries:**

1. **Find slow verification requests:**
   - Service: `soroban-api`
   - Operation: `POST /api/contracts/verify`
   - Min Duration: `500ms`

2. **Trace errors:**
  - Service: `soroban-registry-api`
   - Tags: `error=true`

3. **Indexer polling traces:**
  - Service: `soroban-registry-indexer`
  - Operation contains: `get_latest_ledger` or `get_ledger_operations`

### Dependency Graph

In Jaeger UI, open **Monitor > Dependencies** to view service interaction topology generated from spans.
Trace context propagation uses W3C headers so downstream HTTP calls stay correlated in a single trace.

### Correlation IDs

Every request receives a unique `correlation_id` that appears in:
- Response headers: `X-Correlation-ID`
- Logs: `correlation_id` field
- Traces: `correlation_id` tag

Use correlation IDs to trace a single request across all services and logs.

## Health Check Endpoints

### Application Health

```
GET /health
```

**Response (Healthy):**

```json
{
  "status": "healthy",
  "timestamp": "2026-02-24T12:34:56Z",
  "checks": {
    "database": "ok",
    "cache": "ok",
    "indexer": "ok"
  },
  "uptime_seconds": 86400,
  "version": "1.2.3"
}
```

**Response (Unhealthy):**

```json
{
  "status": "unhealthy",
  "timestamp": "2026-02-24T12:34:56Z",
  "checks": {
    "database": "ok",
    "cache": "degraded",
    "indexer": "failed"
  },
  "errors": [
    "Indexer: unable to reach Stellar RPC after 5 retries"
  ]
}
```

**HTTP Status Codes:**
- `200 OK`: All checks passed
- `503 Service Unavailable`: One or more checks failed

### Readiness Check

```
GET /ready
```

Indicates whether the service is ready to accept traffic (dependencies initialized, migrations complete).

### Liveness Check

```
GET /health/live
```

Simple check that the process is alive (doesn't check dependencies).

## Grafana Dashboards

### Available Dashboards

All dashboards are in `observability/grafana/`:

1. **API Dashboard** (`api_dashboard.json`)
   - Request rate, latency percentiles (P50, P95, P99)
   - Error rate and status code distribution
   - Rate limiting violations

2. **Contracts Dashboard** (`contracts_dashboard.json`)
   - Contract registrations over time
   - Verification success/failure rates
   - Top contracts by interactions

3. **Database Dashboard** (`db_dashboard.json`)
   - Connection pool utilization
   - Query latency and throughput
   - Slow query identification

4. **Publishers Dashboard** (`publishers_dashboard.json`)
   - Active publishers
   - Contracts per publisher
   - Publisher activity trends

5. **SLO Dashboard** (`slo_dashboard.json`)
   - Availability SLI (99.9% target)
   - Latency SLI (P99 < 500ms)
   - Error budget burn rate

### Importing Dashboards

1. Access Grafana: `http://localhost:3000` (default credentials: `admin/admin`)
2. Navigate to **Dashboards** → **Import**
3. Upload JSON file or paste contents
4. Select Prometheus data source
5. Click **Import**

### Dashboard Variables

Most dashboards support variables for filtering:

- `$environment`: production, staging, development
- `$service`: api, indexer, verifier
- `$interval`: Time range for queries (auto, 1m, 5m, 1h)

## Pre-deployment Monitoring Checklist

Before deploying to production, ensure:

- [ ] Prometheus scrapes all service `/metrics` endpoints
- [ ] Alert rules loaded in Prometheus
- [ ] AlertManager configured with notification channels (Slack, PagerDuty)
- [ ] Grafana dashboards imported and displaying data
- [ ] Log aggregation (Loki or ELK) ingesting logs
- [ ] Jaeger receiving traces
- [ ] Health checks responding correctly
- [ ] SLO metrics defined and tracked
- [ ] Runbooks created for all critical alerts (see [ALERTING.md](./ALERTING.md))

## Environment Variables for Observability

```bash
# Logging
LOG_LEVEL=info                          # trace, debug, info, warn, error
LOG_FORMAT=json                         # json or text

# Metrics
METRICS_ENABLED=true
METRICS_PORT=3001

# Tracing
OTEL_ENABLED=true
OTEL_EXPORTER_JAEGER_ENDPOINT=http://jaeger:14268/api/traces
OTEL_SERVICE_NAME=soroban-registry-api
OTEL_TRACES_SAMPLER=parentbased_traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1            # Sample 10% of traces

# Prometheus
PROMETHEUS_PUSHGATEWAY=http://pushgateway:9091
```

## Performance Monitoring Best Practices

### 1. Monitor Golden Signals

Focus on the "Four Golden Signals" of monitoring:

1. **Latency**: Request duration (P50, P95, P99)
2. **Traffic**: Request rate (requests/second)
3. **Errors**: Error rate (errors/requests)
4. **Saturation**: Resource utilization (CPU, memory, connections)

### 2. Set Meaningful Alerts

- Alert on symptoms (user-visible issues), not causes
- Use appropriate thresholds based on SLOs
- Include actionable runbooks (see [ALERTING.md](./ALERTING.md))
- Avoid alert fatigue with proper grouping and inhibition rules

### 3. Establish Baselines

Track metrics over time to understand normal behavior:

```promql
# Baseline request rate (3-week average)
avg_over_time(rate(soroban_http_requests_total[5m])[3w:1h])
```

### 4. Use Dashboards Effectively

- Create role-specific dashboards (developer, SRE, executive)
- Include links to related resources (runbooks, logs, traces)
- Use annotations for deployments and incidents
- Set up TV dashboards for NOC/war rooms

## Troubleshooting

### Issue: Metrics not appearing in Prometheus

**Checks:**
1. Verify service is exposing `/metrics` endpoint: `curl http://api:3001/metrics`
2. Check Prometheus scrape config targets: `http://prometheus:9090/targets`
3. Look for scrape errors in Prometheus logs
4. Verify network connectivity and firewall rules

### Issue: High cardinality metrics causing performance issues

**Symptoms:**
- Prometheus using excessive memory
- Slow query execution
- Missing data points

**Solutions:**
- Reduce label cardinality (avoid high-cardinality labels like IPs, user IDs)
- Increase Prometheus retention and TSDB settings
- Use recording rules for expensive queries

### Issue: Logs not appearing in Loki/Elasticsearch

**Checks:**
1. Verify log file paths in Promtail/Filebeat config
2. Check Promtail/Filebeat is running: `docker ps`
3. Verify Loki/Elasticsearch connectivity
4. Check for parsing errors in Promtail/Filebeat logs

## Related Documentation

- [ALERTING.md](./ALERTING.md) - Alert rules and runbooks
- [ERROR_CODES.md](./ERROR_CODES.md) - Error code reference
- [API_RATE_LIMITING.md](./API_RATE_LIMITING.md) - Rate limiting configuration

## Support

For observability setup issues:
- GitHub Issues: https://github.com/ALIPHATICHYD/Soroban-Registry/issues
- Tag with: `observability`, `monitoring`
