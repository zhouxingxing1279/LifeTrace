# Direct-IP production access

LifeTrace production no longer requires `sslip.io` hostnames. Public clients connect directly to the server IPv4 address while keeping TLS enabled.

## Public endpoints

| Service | Public URL | Notes |
| --- | --- | --- |
| LifeTrace Web | `https://8.148.75.45` | Main browser UI and native `/api/*` routes |
| LifeTrace readiness | `https://8.148.75.45/health/ready` | Public deployment check |
| BeeCount Cloud compatibility | `https://8.148.75.45:8869` | Dedicated stock BeeCount entrypoint |
| BeeCount version | `https://8.148.75.45:8869/api/v1/version` | Expected BeeCount Cloud compatibility response |
| BeeCount WebSocket | `wss://8.148.75.45:8869/ws` | Legacy direct WS path; `/api/v1/ws` is also routed |

Stock BeeCount should be configured with:

- **Server URL:** `https://8.148.75.45:8869`
- **API prefix:** `/api/v1`

Do not append `/api/v1` to the server URL itself.

## Why HTTPS is retained

The LifeTrace production runtime intentionally requires secure cookies and an HTTPS `PUBLIC_WEB_BASE_URL`. Dropping the main deployment to clear-text HTTP would make the production configuration fail validation and would weaken authentication transport security.

The production Caddy image is pinned to Caddy 2.11.4. The Caddyfile explicitly requests the Let's Encrypt `shortlived` ACME profile for the public IPv4 certificate. Caddy persists certificate state in the existing `caddy_data` volume and renews the short-lived certificate automatically.

## Required public ports

The server and cloud-provider security group must allow inbound TCP traffic for:

- `80/tcp` — ACME HTTP challenge and HTTPS redirect support
- `443/tcp` — LifeTrace HTTPS
- `8869/tcp` — BeeCount HTTPS compatibility entrypoint

The Rust service on `8787` remains internal to the Docker network and must not be exposed publicly.

## Routing model

```text
https://8.148.75.45
        |
        +-- /api/* ----------------------> lifetrace-cloud:8787
        +-- /health/* -------------------> lifetrace-cloud:8787
        +-- everything else -------------> LifeTrace browser build

https://8.148.75.45:8869
        |
        +-- /ready -----------------------> /health/ready
        +-- /api/v1/* --------------------> /api/v1/integrations/beecount/compat/*
        +-- /ws --------------------------> /api/v1/integrations/beecount/compat/ws
```

This keeps the stock BeeCount protocol namespace isolated from native LifeTrace routes while both clients continue to use the same Rust backend and account system.

## Production deployment

From `/opt/lifetrace/LifeTrace`:

```bash
git pull --ff-only origin main
bash deploy/cloud/deploy-production.sh
```

The deployment script validates Compose, starts the stack, waits for service health, then verifies both public HTTPS entrypoints with certificate validation enabled.

## Manual verification

```bash
curl -v https://8.148.75.45/health/ready
curl -v https://8.148.75.45:8869/api/v1/version
```

The BeeCount version endpoint should return a JSON payload identifying `BeeCount Cloud`.

WebSocket transport can be checked with an HTTP/1.1 upgrade request against either `/ws` or `/api/v1/ws`; an unauthenticated upgrade may be accepted at the transport layer and then closed by application authentication policy.

## Rollback

If direct-IP certificate issuance or the `8869` entrypoint fails, do not expose `lifetrace-cloud:8787` directly and do not disable TLS verification in clients. Roll back the deployment commit and inspect Caddy logs before changing authentication or BeeCount compatibility code.
