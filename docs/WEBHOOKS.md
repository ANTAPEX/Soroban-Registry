## Webhooks

### Overview

The registry can notify external systems about contract lifecycle events via HTTP webhooks.

Supported event types:

- `contract_verified`
- `contract_deployed`
- `contract_updated`

Webhooks can be:

- global (receive events for all contracts)
- contract-specific (receive events only for a single `contract_id`)

### Register a webhook

`POST /api/v1/webhooks`

```bash
curl -X POST "http://localhost:3001/api/v1/webhooks" \
  -H "Content-Type: application/json" \
  -d '{
    "target_url": "https://example.com/registry/webhook",
    "subscribed_events": ["contract_verified", "contract_updated"],
    "contract_id": null,
    "retry_count": 5
  }'
```

Response returns a `secret` once (base64). Store it securely.

### Delivery signature verification

Each delivery includes:

- `X-Webhook-Timestamp`: unix epoch seconds
- `X-Webhook-Signature`: hex-encoded HMAC-SHA256

Signing input:

```
HMAC_SHA256(secret, "{timestamp}.{raw_body}")
```

Example (Node.js):

```js
import crypto from "crypto";

function verify({ secretBase64, timestamp, rawBody, signatureHex }) {
  const secret = Buffer.from(secretBase64, "base64");
  const expected = crypto
    .createHmac("sha256", secret)
    .update(`${timestamp}.${rawBody}`)
    .digest("hex");
  return crypto.timingSafeEqual(Buffer.from(expected, "hex"), Buffer.from(signatureHex, "hex"));
}
```

### Retries and dead letters

- Transient failures are retried with exponential backoff.
- Retry is triggered on:
  - network errors / timeouts
  - 5xx responses
- Most 4xx responses are not retried.
- After the retry limit is reached, the delivery is placed into the dead letter queue.

### Inspect deliveries

- `GET /api/v1/webhooks/deliveries`
- `GET /api/v1/webhooks/dead-letter`
- `POST /api/v1/webhooks/dead-letter/{id}/retry`

