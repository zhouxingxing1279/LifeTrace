# Deploying BeeCount compatibility with LifeTrace Cloud

BeeCount compatibility is served directly by the unified LifeTrace Rust backend. The legacy `sunxiao0721/beecount-cloud` container is no longer part of the active production stack.

## 1. Configure DNS and environment

Point a hostname such as `finance.example.com` to the LifeTrace server. Add the public compatibility hostname to `deploy/cloud/.env.production` or the Compose environment:

```dotenv
BEECOUNT_DOMAIN=finance.example.com
BEECOUNT_ATTACHMENT_MAX_UPLOAD_BYTES=67108864
```

The non-secret template is `deploy/cloud/beecount.env.example`.

The BeeCount hostname is still required: Caddy accepts the stock BeeCount protocol on that hostname and rewrites the public paths into LifeTrace's internal compatibility namespace.

## 2. Start or upgrade the stack

Use the standard full-Docker deployment described in `docs/docker-deployment.md`:

```bash
cd /opt/lifetrace/LifeTrace
git switch main
git pull --ff-only origin main
cd deploy/cloud

docker compose -f docker-compose.production.yml pull
docker compose -f docker-compose.production.yml up -d
```

No legacy BeeCount Cloud image needs to be pulled or started.

## 3. Routing contract

The production Caddy configuration routes BeeCount clients to `lifetrace-cloud:8787`:

- `/ready` → LifeTrace `/health/ready`
- `/api/v1/*` → `/api/v1/integrations/beecount/compat/*`
- `/ws` → `/api/v1/integrations/beecount/compat/ws`

The primary LifeTrace host and the BeeCount compatibility host therefore share one authentication/data backend and one PostgreSQL deployment.

## 4. Verify

```bash
curl --fail --silent "https://${BEECOUNT_DOMAIN}/ready"
curl --fail --silent "https://${BEECOUNT_DOMAIN}/api/v1/version"
```

Expected readiness response is the normal LifeTrace ready payload.

In the stock BeeCount client, configure the server as `https://finance.example.com` and authenticate with the account supported by the unified compatibility API.

Also verify the native LifeTrace backend:

```bash
docker compose -f deploy/cloud/docker-compose.production.yml exec lifetrace-cloud \
  curl --fail --silent http://127.0.0.1:8787/health/ready
```

## 5. Backup and rollback boundary

Current BeeCount-compatible data belongs to the unified LifeTrace PostgreSQL storage and LifeTrace-managed attachment/storage paths.

If an older deployment created a `beecount_data` Docker volume or exported BeeCount SQLite data, keep that historical data offline until migration/reconciliation is fully signed off. It does not need to remain attached to a running container.

Do not delete historical backup media merely because the legacy service was removed from Compose.

## 6. Security notes

- Only Caddy publishes the public HTTP/HTTPS ports.
- Legacy BeeCount Cloud port 8080 is no longer part of production.
- Caddy obtains and renews TLS certificates for `BEECOUNT_DOMAIN`.
- Keep passwords, database credentials and AI keys in ignored deployment environment files or a secret manager.
- BeeCount and LifeTrace now share the authoritative backend, so account/session compatibility must be validated as part of backend CI rather than by keeping a second server running.
