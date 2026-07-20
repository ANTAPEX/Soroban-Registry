## API Versioning

### Strategy

- URL-based versioning:
  - v1: `/api/v1/...`
  - v2: `/api/v2/...`
- Backward-compatible legacy alias:
  - `/api/...` (deprecated alias for v1)

### Version headers

All versioned responses include:

- `X-API-Version: v1` or `X-API-Version: v2`

Legacy alias responses also include:

- `Deprecation: true`
- `Sunset: Wed, 31 Dec 2026 00:00:00 GMT`
- `Warning: 299 - "Deprecated API path. Use /api/v1 or /api/v2."`

### Lightweight analytics

`GET /api/v1/stats` includes counters:

- `api_versions.v1_calls`
- `api_versions.v2_calls`
- `api_versions.deprecated_alias_calls`

### Swagger/OpenAPI

- Swagger UI: `GET /api/docs`
- v1 spec: `GET /api/v1/openapi.json`
- v2 spec: `GET /api/v2/openapi.json`
- default spec: `GET /openapi.json` (v1)

