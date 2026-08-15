# Direct-IP production access

LifeTrace production on the personal server uses the public IPv4 address directly and plain HTTP. This intentionally removes DNS, TLS, certificate, ACME and SNI dependencies from the deployment path.

## Public endpoints

| Service | Public URL | Notes |
| --- | --- | --- |
| LifeTrace Web | `http://8.148.75.45` | Main browser UI and native `/api/*` routes |
| LifeTrace readiness | `http://8.148.75.45/health/ready` | Public deployment check |
| BeeCount Cloud compatibility | `http://8.148.75.45:8869` | Dedicated stock BeeCount entrypoint |
| BeeCount version | `http://8.148.75.45:8869/api/v1/version` | Expected BeeCount Cloud compatibility response |
| BeeCount WebSocket | `ws://8.148.75.45:8869/ws` | Legacy direct WS path; `/api/v1/ws` is also routed |

Stock BeeCount should be configured with:

- **Server URL:** `http://8.148.75.45:8869`
- **API prefix:** `/api/v1`

Do not append `/api/v1` to the server URL itself.

## Deployment profile

Caddy is retained only as a small HTTP static-file server and reverse proxy. It does not manage HTTPS or certificates. The public stack exposes only:

- `80/tcp` — LifeTrace HTTP
- `8869/tcp` — BeeCount HTTP compatibility entrypoint

Port `443` is not required. The Rust service on `8787` remains internal to the Docker network and must not be exposed publicly.

The HTTP self-host profile explicitly disables development authentication while using a non-production validation mode so the existing backend does not require HTTPS-only secure cookies. The browser session cookie is named `lifetrace_session` and is intentionally not marked `Secure` for this HTTP-only deployment.

> This profile is deliberately optimized for simplicity on a personal server. HTTP traffic is not encrypted in transit. If the service is later exposed to untrusted users or networks, switch back to a normal domain-backed HTTPS deployment instead of weakening TLS verification.

## Routing model

```text
http://8.148.75.45
        |
        +-- /api/* ----------------------> lifetrace-cloud:8787
        +-- /health/* -------------------> lifetrace-cloud:8787
        +-- everything else -------------> LifeTrace browser build

http://8.148.75.45:8869
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

The deployment script validates Compose, starts the stack, waits for service health, then verifies both public HTTP entrypoints. There is no certificate wait or ACME validation step.

## Manual verification

```bash
curl -v http://8.148.75.45/health/ready
curl -v http://8.148.75.45:8869/api/v1/version
```

The BeeCount version endpoint should return a JSON payload identifying `BeeCount Cloud`.

WebSocket transport can be checked with an HTTP/1.1 upgrade request against either `/ws` or `/api/v1/ws`; an unauthenticated upgrade may be accepted at the transport layer and then closed by application authentication policy.

## Failure diagnosis

If deployment reports connectivity failures, inspect:

```bash
docker compose --env-file deploy/cloud/.env \
  -f deploy/cloud/docker-compose.production.yml ps -a

docker compose --env-file deploy/cloud/.env \
  -f deploy/cloud/docker-compose.production.yml logs --tail=150 caddy
```

Also verify Alibaba Cloud and any host firewall allow inbound TCP `80` and `8869`.
