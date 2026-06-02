# API CORS and CSRF Policy

The API reads allowed browser origins from `CORS_ALLOWED_ORIGINS`, falling back
to `ALLOWED_ORIGINS`, then to:

```text
http://localhost:3000,https://soroban-registry.vercel.app
```

Allowed origins receive credentials-enabled CORS responses, preflight support
for `GET`, `HEAD`, `POST`, `PUT`, `PATCH`, `DELETE`, and `OPTIONS`, and exposed
request/rate-limit/CSRF headers.

Browser mutation requests are rejected when:

- `Origin` is not in the configured allow-list.
- `Sec-Fetch-Site` reports `cross-site`.
- A cookie or browser origin is present but `X-CSRF-Token` does not match the
  `sr_csrf` same-site cookie.

Clients can fetch a CSRF token from:

```text
GET /api/auth/csrf
```

The response sets `sr_csrf` with `SameSite=Lax`, `HttpOnly`, `Path=/`, and
`Secure` by default. Set `CSRF_COOKIE_SECURE=false` for local non-HTTPS
development only. Set `CSRF_COOKIE_SAMESITE=strict|lax|none` to tune cookie
same-site behavior.
