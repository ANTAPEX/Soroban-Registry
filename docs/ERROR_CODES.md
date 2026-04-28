# Error Handling and Error Codes Reference

## Overview

This document provides a comprehensive reference of all error codes returned by the Soroban Registry API, their meanings, causes, and how to handle them in client applications.

## Error Response Format

All API errors follow a consistent JSON structure:

```json
{
  "code": "INVALID_FILTER",
  "message": "Human-readable error description",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "details": {
    "reason": "INVALID_FILTER",
    "field": "network",
    "allowed": ["mainnet", "testnet", "futurenet"]
  },
  "error_code": "BAD_REQUEST",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-02-24T12:34:56Z"
}
```

`code` is the endpoint-specific machine-readable error code (for example `CONTRACT_NOT_FOUND`, `INVALID_FILTER`, `UNAUTHORIZED`, `RATE_LIMITED`).

`error_code` remains available as a coarse-grained class derived from HTTP status.

Supported top-level `error_code` values are standardized and derived from HTTP status classes:

```text
BAD_REQUEST
UNAUTHORIZED
FORBIDDEN
NOT_FOUND
CONFLICT
UNPROCESSABLE_ENTITY
PAYLOAD_TOO_LARGE
RATE_LIMITED
INTERNAL_ERROR
```

`details` contains endpoint-specific context for client handling, for example validation field errors:

```json
{
  "error_code": "BAD_REQUEST",
  "message": "Validation failed for 2 fields",
  "details": {
    "reason": "VALIDATION_ERROR",
    "field_errors": [
      {"field": "contract_id", "message": "Invalid format"},
      {"field": "network", "message": "Unsupported network"}
    ],
    "correlation_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `code` | string | Endpoint-specific machine-readable error code |
| `message` | string | Human-readable error description |
| `request_id` | string | Request tracing identifier |
| `details` | object | Additional context (optional, varies by endpoint) |
| `error_code` | string | Standardized error class derived from HTTP status |
| `correlation_id` | string | Backward-compatible alias for request_id |
| `timestamp` | string | ISO 8601 timestamp when error occurred |

## HTTP Status Codes

### 2xx Success

| Code | Status | Meaning |
|------|--------|---------|
| 200 | OK | Request succeeded |
| 201 | Created | Resource created successfully |
| 202 | Accepted | Request accepted for async processing |
| 204 | No Content | Request succeeded with no response body |

### 4xx Client Errors

Client errors indicate the request was invalid. **Users can fix these errors.**

#### 400 Bad Request

The request is malformed or contains invalid parameters.

**Common Error Codes:**

##### ERR_INVALID_INPUT

```json
{
  "error": "ERR_INVALID_INPUT",
  "message": "Invalid input parameters",
  "code": 400,
  "details": {
    "field": "contract_id",
    "reason": "Contract ID must be 56 characters long"
  }
}
```

**Causes:**
- Invalid contract ID format
- Missing required fields
- Invalid data types (string instead of number)
- Out-of-range values

**Client Action:** Validate input before sending. Check API documentation for correct format.

**Example (Python):**
```python
def validate_contract_id(contract_id: str) -> bool:
    return len(contract_id) == 56 and contract_id[0] == 'C'

if not validate_contract_id(contract_id):
    raise ValueError("Invalid contract ID format")
```

---

##### ERR_INVALID_NETWORK

```json
{
  "error": "ERR_INVALID_NETWORK",
  "message": "Invalid network specified",
  "code": 400,
  "details": {
    "network": "mainnett",
    "valid_networks": ["mainnet", "testnet", "futurenet"]
  }
}
```

**Causes:**
- Typo in network name
- Unsupported network

**Client Action:** Use one of: `mainnet`, `testnet`, `futurenet`

---

##### ERR_INVALID_PAGINATION

```json
{
  "error": "ERR_INVALID_PAGINATION",
  "message": "Invalid pagination parameters",
  "code": 400,
  "details": {
    "limit": 10000,
    "max_limit": 1000
  }
}
```

**Causes:**
- `limit` exceeds maximum (1000)
- `offset` is negative
- Invalid cursor format

**Client Action:** Use `limit <= 1000` and valid pagination parameters.

---

##### ERR_MALFORMED_JSON

```json
{
  "error": "ERR_MALFORMED_JSON",
  "message": "Request body is not valid JSON",
  "code": 400,
  "details": {
    "parse_error": "Expected ',' at line 5, column 12"
  }
}
```

**Causes:**
- Missing quotes, commas, or brackets in JSON
- Trailing commas (invalid in JSON)
- Invalid escape sequences

**Client Action:** Validate JSON before sending. Use `JSON.stringify()` or equivalent.

---

#### 401 Unauthorized

Authentication is required but was not provided or is invalid.

##### ERR_MISSING_AUTH

```json
{
  "error": "ERR_MISSING_AUTH",
  "message": "Authentication required for this endpoint",
  "code": 401
}
```

**Causes:**
- No `Authorization` header provided
- Endpoint requires authentication

**Client Action:** Include valid API key in `Authorization` header.

**Example:**
```http
GET /api/contracts/private
Authorization: Bearer your-api-key-here
```

---

##### ERR_INVALID_TOKEN

```json
{
  "error": "ERR_INVALID_TOKEN",
  "message": "Invalid or expired authentication token",
  "code": 401
}
```

**Causes:**
- API key is invalid
- Token has expired
- Token was revoked

**Client Action:** Obtain a new API key or refresh token.

---

#### 403 Forbidden

Client is authenticated but doesn't have permission for this resource.

##### ERR_FORBIDDEN

```json
{
  "error": "ERR_FORBIDDEN",
  "message": "You don't have permission to access this resource",
  "code": 403,
  "details": {
    "required_permission": "admin:write"
  }
}
```

**Causes:**
- Insufficient permissions
- Trying to modify another user's resource
- IP address blocked

**Client Action:** Request appropriate permissions or contact administrator.

---

#### 404 Not Found

The requested resource doesn't exist.

##### ERR_CONTRACT_NOT_FOUND

```json
{
  "error": "ERR_CONTRACT_NOT_FOUND",
  "message": "Contract not found",
  "code": 404,
  "details": {
    "contract_id": "CDLZFC3...",
    "network": "mainnet"
  }
}
```

**Causes:**
- Contract ID doesn't exist
- Contract deleted
- Wrong network specified

**Client Action:** Verify contract ID and network. Check if contract exists on-chain.

---

##### ERR_PUBLISHER_NOT_FOUND

```json
{
  "error": "ERR_PUBLISHER_NOT_FOUND",
  "message": "Publisher not found",
  "code": 404,
  "details": {
    "publisher_id": "pub_abc123"
  }
}
```

**Causes:**
- Publisher ID invalid
- Publisher account deleted

**Client Action:** Verify publisher ID or search by name.

---

##### ERR_VERIFICATION_NOT_FOUND

```json
{
  "error": "ERR_VERIFICATION_NOT_FOUND",
  "message": "Verification record not found",
  "code": 404,
  "details": {
    "verification_id": "ver_xyz789"
  }
}
```

**Causes:**
- Verification ID doesn't exist
- Verification expired

**Client Action:** Check verification ID or initiate new verification.

---

#### 409 Conflict

Request conflicts with current state of the resource.

##### ERR_ALREADY_EXISTS

```json
{
  "error": "ERR_ALREADY_EXISTS",
  "message": "Resource already exists",
  "code": 409,
  "details": {
    "resource_type": "contract",
    "contract_id": "CDLZFC3..."
  }
}
```

**Causes:**
- Attempting to create duplicate contract entry
- Contract already verified

**Client Action:** Use UPDATE endpoint instead of CREATE, or check existing resource.

---

##### ERR_VERIFICATION_IN_PROGRESS

```json
{
  "error": "ERR_VERIFICATION_IN_PROGRESS",
  "message": "Verification already in progress for this contract",
  "code": 409,
  "details": {
    "verification_id": "ver_abc123",
    "status": "pending"
  }
}
```

**Causes:**
- Previous verification still running

**Client Action:** Wait for existing verification to complete or cancel it first.

---

#### 422 Unprocessable Entity

Request is well-formed but semantically invalid.

##### ERR_INVALID_CONTRACT_SOURCE

```json
{
  "error": "ERR_INVALID_CONTRACT_SOURCE",
  "message": "Contract source code is invalid",
  "code": 422,
  "details": {
    "reason": "Missing Cargo.toml",
    "required_files": ["Cargo.toml", "src/lib.rs"]
  }
}
```

**Causes:**
- Incomplete source code
- Missing required files
- Invalid file structure

**Client Action:** Ensure source includes all required files.

---

##### ERR_UNSUPPORTED_COMPILER

```json
{
  "error": "ERR_UNSUPPORTED_COMPILER",
  "message": "Compiler version not supported",
  "code": 422,
  "details": {
    "requested_version": "19.0.0",
    "supported_versions": ["20.0.0", "20.5.0", "21.0.0", "21.2.0"]
  }
}
```

**Causes:**
- Requesting unsupported compiler version

**Client Action:** Use a supported compiler version.

---

#### 429 Too Many Requests

Rate limit exceeded. See [API Rate Limiting](./API_RATE_LIMITING.md).

##### ERR_RATE_LIMIT_EXCEEDED

```json
{
  "error": "ERR_RATE_LIMIT_EXCEEDED",
  "message": "Too many requests. Please retry after the indicated time.",
  "code": 429,
  "timestamp": "2026-02-24T12:34:56Z",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response Headers:**
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 42
Retry-After: 42
```

**Causes:**
- Exceeded rate limit for your tier
- Too many requests in time window

**Client Action:**
1. Wait `Retry-After` seconds before retrying
2. Implement exponential backoff
3. Monitor `X-RateLimit-Remaining` header
4. Request higher tier if needed

**Example (Python):**
```python
import time

response = requests.get(url)
if response.status_code == 429:
    retry_after = int(response.headers.get('Retry-After', 60))
    print(f"Rate limited. Waiting {retry_after}s...")
    time.sleep(retry_after)
    response = requests.get(url)  # Retry
```

---

### 5xx Server Errors

Server errors indicate a problem on the server side. **Users should retry with exponential backoff.**

#### 500 Internal Server Error

Unexpected server error.

##### ERR_INTERNAL_SERVER_ERROR

```json
{
  "error": "ERR_INTERNAL_SERVER_ERROR",
  "message": "An unexpected error occurred",
  "code": 500,
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Causes:**
- Unhandled exception
- Bug in server code
- Unexpected data state

**Client Action:**
1. Retry with exponential backoff (3-5 attempts)
2. If persists, report issue with `correlation_id`

**Example (JavaScript):**
```javascript
async function retryOnError(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.status === 500 && i < maxRetries - 1) {
        const delay = Math.pow(2, i) * 1000; // Exponential backoff
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }
      throw error;
    }
  }
}
```

---

##### ERR_DATABASE_ERROR

```json
{
  "error": "ERR_DATABASE_ERROR",
  "message": "Database operation failed",
  "code": 500,
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Causes:**
- Database connection lost
- Database timeout
- Database constraint violation

**Client Action:** Retry after brief delay. If persists, report issue.

---

##### ERR_COMPILATION_ERROR

```json
{
  "error": "ERR_COMPILATION_ERROR",
  "message": "Contract compilation failed",
  "code": 500,
  "details": {
    "compiler_output": "error: could not compile `contract`..."
  }
}
```

**Causes:**
- Internal compilation error
- Compiler crash
- Build environment issue

**Client Action:** Check source code validity. If source is correct, report issue.

---

#### 502 Bad Gateway

Upstream service (e.g., Stellar RPC) is unavailable.

##### ERR_RPC_ERROR

```json
{
  "error": "ERR_RPC_ERROR",
  "message": "Failed to communicate with Stellar RPC",
  "code": 502,
  "details": {
    "rpc_endpoint": "https://soroban-testnet.stellar.org",
    "reason": "Connection timeout"
  }
}
```

**Causes:**
- Stellar RPC is down
- Network connectivity issues
- RPC timeout

**Client Action:** Retry with exponential backoff. Check Stellar network status.

---

#### 503 Service Unavailable

Service is temporarily unavailable.

##### ERR_SERVICE_UNAVAILABLE

```json
{
  "error": "ERR_SERVICE_UNAVAILABLE",
  "message": "Service temporarily unavailable",
  "code": 503,
  "details": {
    "reason": "Maintenance in progress",
    "retry_after": 300
  }
}
```

**Response Headers:**
```http
Retry-After: 300
```

**Causes:**
- Scheduled maintenance
- System overload
- Deployment in progress

**Client Action:** Wait `Retry-After` seconds and retry.

---

##### ERR_MAINTENANCE_MODE

```json
{
  "error": "ERR_MAINTENANCE_MODE",
  "message": "System is in maintenance mode",
  "code": 503,
  "details": {
    "estimated_duration": "30 minutes",
    "started_at": "2026-02-24T12:00:00Z"
  }
}
```

**Causes:**
- Scheduled maintenance window

**Client Action:** Wait for maintenance to complete. Check status page.

---

#### 504 Gateway Timeout

Upstream service took too long to respond.

##### ERR_TIMEOUT

```json
{
  "error": "ERR_TIMEOUT",
  "message": "Request timeout",
  "code": 504,
  "details": {
    "timeout_seconds": 30,
    "operation": "contract_verification"
  }
}
```

**Causes:**
- Operation took longer than timeout
- Slow upstream service

**Client Action:** Retry request. For verification, consider using async endpoint.

---

## Verification-Specific Errors

### ERR_BYTECODE_MISMATCH

```json
{
  "error": "ERR_BYTECODE_MISMATCH",
  "message": "Compiled bytecode does not match on-chain bytecode",
  "code": 422,
  "details": {
    "expected_hash": "a3f2b8c9d1e4f5a6b7c8d9e0...",
    "actual_hash": "9f1a2b3c4d5e6f7a8b9c0d1e...",
    "possible_causes": [
      "Wrong compiler version",
      "Different optimization level",
      "Dependency version mismatch"
    ]
  }
}
```

**Client Action:** See [Verification Troubleshooting](./VERIFICATION_TROUBLESHOOTING.md).

### ERR_COMPILATION_FAILED

```json
{
  "error": "ERR_COMPILATION_FAILED",
  "message": "Source code failed to compile",
  "code": 422,
  "details": {
    "compiler_output": "error[E0425]: cannot find function `transfer` in this scope\n --> src/lib.rs:42:5"
  }
}
```

**Client Action:** Fix compilation errors in source code.

### ERR_ABI_MISMATCH

```json
{
  "error": "ERR_ABI_MISMATCH",
  "message": "Contract ABI does not match declared interface",
  "code": 422,
  "details": {
    "function": "transfer",
    "expected_signature": "(Address, Address, i128)",
    "actual_signature": "(Address, i128)"
  }
}
```

**Client Action:** Ensure source matches deployed version.

## Error Handling Best Practices

### 1. Always Check Status Codes

```python
response = requests.get(url)
if response.status_code >= 400:
    error_data = response.json()
    handle_error(error_data)
```

### 2. Parse Error Response

```javascript
async function callApi(url) {
  const response = await fetch(url);

  if (!response.ok) {
    const error = await response.json();
    throw new ApiError(
      error.error,
      error.message,
      error.correlation_id
    );
  }

  return response.json();
}
```

### 3. Implement Retry Logic

**Retry on:**
- 5xx errors (server issues)
- 429 (rate limit - with backoff)
- 502, 503, 504 (upstream/availability issues)

**Don't retry on:**
- 4xx errors (except 429) - these require fixing the request

```rust
async fn call_with_retry<T>(
    operation: impl Fn() -> Result<T>,
    max_retries: u32,
) -> Result<T> {
    for attempt in 0..max_retries {
        match operation() {
            Ok(value) => return Ok(value),
            Err(e) if e.is_retryable() && attempt < max_retries - 1 => {
                let delay = Duration::from_secs(2_u64.pow(attempt));
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### 4. Log Correlation IDs

Always log `correlation_id` for debugging:

```python
try:
    response = api_client.get_contract(contract_id)
except ApiError as e:
    logger.error(
        f"API error: {e.error_code}",
        extra={
            "correlation_id": e.correlation_id,
            "contract_id": contract_id
        }
    )
    raise
```

### 5. Handle Rate Limits Proactively

```javascript
class ApiClient {
  async request(url) {
    const response = await fetch(url);

    // Check rate limit headers
    const remaining = parseInt(response.headers.get('X-RateLimit-Remaining'));
    if (remaining < 10) {
      console.warn(`Only ${remaining} requests remaining!`);
    }

    if (response.status === 429) {
      const retryAfter = parseInt(response.headers.get('Retry-After'));
      await this.sleep(retryAfter * 1000);
      return this.request(url); // Retry
    }

    return response.json();
  }
}
```

### 6. Provide User-Friendly Messages

Don't expose raw error codes to end users:

```python
USER_FRIENDLY_MESSAGES = {
    "ERR_CONTRACT_NOT_FOUND": "We couldn't find that contract. Please check the contract ID.",
    "ERR_RATE_LIMIT_EXCEEDED": "Too many requests. Please wait a moment and try again.",
    "ERR_INTERNAL_SERVER_ERROR": "Something went wrong on our end. Please try again later.",
}

def format_error_for_user(error_code):
    return USER_FRIENDLY_MESSAGES.get(
        error_code,
        "An unexpected error occurred. Please try again."
    )
```

## Code Examples by Language

### Python

```python
import requests
import time
from typing import Optional

class SorobanRegistryClient:
    def __init__(self, base_url: str, api_key: Optional[str] = None):
        self.base_url = base_url
        self.session = requests.Session()
        if api_key:
            self.session.headers['Authorization'] = f'Bearer {api_key}'

    def get_contract(self, contract_id: str):
        try:
            response = self.session.get(f'{self.base_url}/api/contracts/{contract_id}')
            response.raise_for_status()
            return response.json()
        except requests.HTTPError as e:
            if e.response.status_code == 404:
                raise ContractNotFoundError(contract_id)
            elif e.response.status_code == 429:
                retry_after = int(e.response.headers.get('Retry-After', 60))
                raise RateLimitError(retry_after)
            elif e.response.status_code >= 500:
                error_data = e.response.json()
                raise ServerError(error_data['correlation_id'])
            else:
                raise
```

### JavaScript/TypeScript

```typescript
interface ApiError {
  error: string;
  message: string;
  code: number;
  correlation_id: string;
  details?: any;
}

class SorobanRegistryClient {
  constructor(
    private baseUrl: string,
    private apiKey?: string
  ) {}

  async getContract(contractId: string) {
    const response = await fetch(
      `${this.baseUrl}/api/contracts/${contractId}`,
      {
        headers: this.apiKey
          ? { 'Authorization': `Bearer ${this.apiKey}` }
          : {}
      }
    );

    if (!response.ok) {
      const error: ApiError = await response.json();

      switch (error.code) {
        case 404:
          throw new ContractNotFoundError(contractId);
        case 429:
          const retryAfter = parseInt(response.headers.get('Retry-After') || '60');
          throw new RateLimitError(retryAfter);
        case 500:
        case 502:
        case 503:
          throw new ServerError(error.correlation_id);
        default:
          throw new ApiError(error.error, error.message);
      }
    }

    return response.json();
  }
}
```

### Rust

```rust
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ApiError {
    error: String,
    message: String,
    code: u16,
    correlation_id: String,
}

pub async fn get_contract(
    client: &reqwest::Client,
    base_url: &str,
    contract_id: &str,
) -> Result<Contract, Box<dyn std::error::Error>> {
    let url = format!("{}/api/contracts/{}", base_url, contract_id);
    let response = client.get(&url).send().await?;

    match response.status() {
        StatusCode::OK => {
            let contract = response.json::<Contract>().await?;
            Ok(contract)
        }
        StatusCode::NOT_FOUND => {
            Err(format!("Contract {} not found", contract_id).into())
        }
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            Err(format!("Rate limited. Retry after {}s", retry_after).into())
        }
        _ if response.status().is_server_error() => {
            let error = response.json::<ApiError>().await?;
            Err(format!(
                "Server error: {} (correlation_id: {})",
                error.message, error.correlation_id
            )
            .into())
        }
        _ => {
            let error = response.json::<ApiError>().await?;
            Err(error.message.into())
        }
    }
}
```

## Related Documentation

- [API Rate Limiting](./API_RATE_LIMITING.md) - Rate limit handling
- [Verification Troubleshooting](./VERIFICATION_TROUBLESHOOTING.md) - Verification error fixes
- [API Advanced Features](./API_ADVANCED_FEATURES.md) - Advanced API usage

## Support

For error-related questions:
- GitHub Issues: https://github.com/ALIPHATICHYD/Soroban-Registry/issues
- Tag with: `api`, `errors`
