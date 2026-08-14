# Docker deployment

LifeTrace production deployment is image-first. The server does not build the browser application and does not need Node.js or npm.

## Published images

GitHub Actions publishes two images from `main`:

- `ghcr.io/zhouxingxing1279/lifetrace-cloud:main` — API server, database migrator, mail worker, and execution maintenance worker.
- `ghcr.io/zhouxingxing1279/lifetrace-web:main` — browser Web build, Photo Challenge PWA static files, and the production Caddy configuration.

The Web image is built by `deploy/cloud/Dockerfile.web`. Its Node stage runs `npm ci` and `npm run browser:build`; the runtime stage is Caddy and contains both `/srv` and `/srv-photo-challenge`.

## Production services

`deploy/cloud/docker-compose.production.yml` runs:

- PostgreSQL 16
- `lifetrace-migrate` as a one-shot migration job
- `lifetrace-cloud`
- `lifetrace-mail-worker`
- `lifetrace-execution-worker`
- the packaged `lifetrace-web` Caddy image

Stock BeeCount clients use the compatibility routes implemented by LifeTrace Cloud. Legacy `sunxiao0721/beecount-cloud` is not part of the active Compose stack.

Historical BeeCount data exports or old Docker volumes may be retained offline as a rollback/archive boundary, but they are not required to serve current BeeCount clients.

## Server configuration

Production secrets remain outside Git in `deploy/cloud/.env.production`. Docker Compose interpolation values such as `POSTGRES_PASSWORD` may be placed in `deploy/cloud/.env`.

The PostgreSQL password in `.env` must match the password encoded in the production `DATABASE_URL` for the `lifetrace` user.

Example:

```dotenv
# deploy/cloud/.env
POSTGRES_PASSWORD=<same-password-used-by-DATABASE_URL>
```

Do not commit either production secret file.

## Deploy or upgrade

From the checked-out repository on the server:

```bash
cd /opt/lifetrace/LifeTrace
git switch main
git pull --ff-only origin main
cd deploy/cloud

docker compose -f docker-compose.production.yml pull
docker compose -f docker-compose.production.yml up -d
```

No `npm ci`, `npm run browser:build`, or host bind mount is required.

## Verification

```bash
docker compose -f docker-compose.production.yml ps
```

Expected long-running services are PostgreSQL, Cloud, mail worker, execution worker, and Caddy/Web. `lifetrace-migrate` is expected to exit successfully after migrations complete.

Check migration and worker logs when upgrading:

```bash
docker compose -f docker-compose.production.yml ps -a lifetrace-migrate
docker compose -f docker-compose.production.yml logs --tail=100 lifetrace-migrate
docker compose -f docker-compose.production.yml logs --tail=100 lifetrace-execution-worker
```

The Cloud port is intentionally internal to the Docker network in production. Validate readiness through the container or public Caddy route rather than expecting host port `127.0.0.1:8787` to be published.

```bash
docker compose -f docker-compose.production.yml exec lifetrace-cloud \
  curl --fail --silent http://127.0.0.1:8787/health/ready
```

For the public site, verify both the primary LifeTrace host and the BeeCount compatibility host after DNS/TLS are active.

## Image release gates

- `.github/workflows/cloud-image.yml` builds and publishes the Cloud image.
- `.github/workflows/web-image.yml` builds the full Web/Caddy image and publishes `:main` only from `main`.
- Pull requests build the Web image without publishing it, so Docker packaging failures block the change before merge.
