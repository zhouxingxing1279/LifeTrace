# Deploying BeeCount compatibility with LifeTrace Cloud

## 1. Configure DNS and environment

Point a hostname such as `finance.example.com` to the LifeTrace server. Add the
following values to `deploy/cloud/.env.production` (the file is intentionally
git-ignored):

```dotenv
BEECOUNT_DOMAIN=finance.example.com
BEECOUNT_CLOUD_IMAGE=sunxiao0721/beecount-cloud:latest

# Optional. Leave both unset for a generated first administrator.
BEECOUNT_ADMIN_EMAIL=you@example.com
BEECOUNT_ADMIN_PASSWORD=replace-with-a-long-random-password

# Phase 2 read adapter. For a personal deployment these can initially match
# the BeeCount administrator credentials. The account must not require 2FA.
BEECOUNT_ADAPTER_ENABLED=true
BEECOUNT_ADAPTER_BASE_URL=http://beecount-cloud:8080/
BEECOUNT_ADAPTER_EMAIL=you@example.com
BEECOUNT_ADAPTER_PASSWORD=replace-with-a-long-random-password
BEECOUNT_ADAPTER_LIFETRACE_USER_ID=<userId returned by LifeTrace /api/v1/auth/me>
```

The complete non-secret template is `deploy/cloud/beecount.env.example`.

## 2. Start or upgrade the stack

From the repository root:

```bash
docker compose --env-file deploy/cloud/.env.production \
  -f deploy/cloud/docker-compose.production.yml pull
docker compose --env-file deploy/cloud/.env.production \
  -f deploy/cloud/docker-compose.production.yml up -d --wait
```

If administrator credentials were not configured, retrieve the generated
credentials immediately after first boot:

```bash
docker compose --env-file deploy/cloud/.env.production \
  -f deploy/cloud/docker-compose.production.yml logs beecount-cloud
```

BeeCount Cloud also persists the generated credential fallback inside its
private `/data` volume. Log in and change the password before normal use.

## 3. Verify

```bash
curl --fail --silent "https://${BEECOUNT_DOMAIN}/ready"
curl --fail --silent "https://${BEECOUNT_DOMAIN}/api/v1/version"
```

Expected readiness response:

```json
{"status":"ready"}
```

In the BeeCount iOS app, select **BeeCount Cloud**, enter
`https://finance.example.com`, then sign in with the administrator/user account.

After logging in to LifeTrace, verify the adapter through the LifeTrace origin:

```bash
curl --fail --silent \
  -H "Authorization: Bearer ${LIFETRACE_ACCESS_TOKEN}" \
  "https://example.com/api/v1/integrations/beecount/status"

curl --fail --silent \
  -H "Authorization: Bearer ${LIFETRACE_ACCESS_TOKEN}" \
  "https://example.com/api/v1/integrations/beecount/ledgers"
```

The ledger snapshot endpoint is:

```text
GET /api/v1/integrations/beecount/ledgers/{sourceLedgerId}/snapshot?limit=200&offset=0
```

It is read-only and returns integer-cent, `beecount:`-namespaced data. It does
not copy rows into LifeTrace sync storage.

LifeTrace Web users bound to the adapter can open **资产与账单 → BeeCount
云账本** (route `/finance/beecount`). The page uses the existing LifeTrace Web
session; it never asks the browser for BeeCount credentials.

## 4. Backup and restore boundary

BeeCount data is not stored in LifeTrace PostgreSQL. Back up both stores:

- `lifetrace_pgdata` for LifeTrace Cloud;
- `beecount_data` for BeeCount accounts, SQLite database, attachments, secrets
  and backup configuration.

Do not delete `beecount_data` during routine upgrades. Use BeeCount Cloud's
built-in encrypted backup feature or archive the Docker volume while the service
is stopped.

## 5. Security notes

- Only Caddy publishes network ports; BeeCount's port 8080 stays private.
- Public self-registration is disabled.
- Caddy obtains and renews TLS certificates for `BEECOUNT_DOMAIN`.
- Keep administrator passwords and optional AI keys only in the ignored
  `.env.production` file or a deployment secret manager.
- For reproducible upgrades, replace `latest` with a reviewed version tag or
  image digest.
