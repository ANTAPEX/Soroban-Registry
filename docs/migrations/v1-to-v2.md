## v1 to v2 API Migration

### Summary

- v1 and v2 run side-by-side.
- Prefer versioned paths:
  - v1: `/api/v1/...`
  - v2: `/api/v2/...`
- Legacy paths without an explicit version remain available for backward compatibility:
  - `/api/...` (deprecated alias for v1)

### Deprecation behavior

Requests to legacy `/api/...` paths include:

- `Deprecation: true`
- `Sunset: Wed, 31 Dec 2026 00:00:00 GMT`
- `Warning: 299 - "Deprecated API path. Use /api/v1 or /api/v2."`

### Recommended upgrade

Replace:

- `GET /api/contracts` → `GET /api/v1/contracts` (or `GET /api/v2/contracts`)
- `POST /api/contracts/verify` → `POST /api/v1/contracts/verify` (or `POST /api/v2/contracts/verify`)
- `GET /api/stats` → `GET /api/v1/stats` (or `GET /api/v2/stats`)

### Example (contracts list)

```bash
curl "http://localhost:3001/api/v1/contracts?page=1&limit=20"
```

### Example (verification)

```bash
curl -X POST "http://localhost:3001/api/v1/contracts/verify" \
  -H "Content-Type: application/json" \
  -d '{
    "contract_id": "CABC...XYZ",
    "source_code": "...",
    "build_params": {},
    "compiler_version": "soroban-cli 21.0.0"
  }'
```

