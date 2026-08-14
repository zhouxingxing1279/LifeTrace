#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_ENV_FILE="${SCRIPT_DIR}/.env"
WAIT_SECONDS="${LIFETRACE_DEPLOY_WAIT_SECONDS:-180}"
SKIP_GIT_UPDATE="false"

usage() {
  cat <<'EOF'
Usage: bash deploy/cloud/deploy-production.sh [--skip-git-update]

Deploy the current LifeTrace production stack from published GHCR images.

By default the script:
  1. verifies the server checkout is clean;
  2. switches to main and fast-forwards from origin/main;
  3. validates Docker Compose configuration;
  4. pulls the Cloud and Web images;
  5. starts the production stack with orphan cleanup;
  6. verifies migration completion and core service health.

Options:
  --skip-git-update  Deploy the current checkout without fetching/switching branches.
                     Intended for pinned/manual rollback checkouts.
  -h, --help         Show this help text.
EOF
}

log() {
  printf '[LifeTrace deploy] %s\n' "$*"
}

fail() {
  printf '[LifeTrace deploy] ERROR: %s\n' "$*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

compose() {
  docker compose --env-file "${COMPOSE_ENV_FILE}" -f "${COMPOSE_FILE}" "$@"
}

container_id() {
  compose ps -a -q "$1" | head -n 1
}

wait_for_migration() {
  local service="lifetrace-migrate"
  local cid=""
  local deadline=$((SECONDS + WAIT_SECONDS))

  log "waiting for database migration job"
  while (( SECONDS < deadline )); do
    cid="$(container_id "${service}")"
    if [[ -n "${cid}" ]]; then
      local status
      status="$(docker inspect --format '{{.State.Status}}' "${cid}")"
      if [[ "${status}" == "exited" ]]; then
        local exit_code
        exit_code="$(docker inspect --format '{{.State.ExitCode}}' "${cid}")"
        [[ "${exit_code}" == "0" ]] || {
          compose logs --tail=100 "${service}" >&2 || true
          fail "${service} exited with code ${exit_code}"
        }
        log "database migrations completed successfully"
        return 0
      fi
      if [[ "${status}" == "dead" ]]; then
        compose logs --tail=100 "${service}" >&2 || true
        fail "${service} entered dead state"
      fi
    fi
    sleep 2
  done

  compose logs --tail=100 "${service}" >&2 || true
  fail "timed out waiting for ${service} after ${WAIT_SECONDS}s"
}

wait_for_service() {
  local service="$1"
  local health_required="$2"
  local cid=""
  local deadline=$((SECONDS + WAIT_SECONDS))

  log "waiting for ${service}"
  while (( SECONDS < deadline )); do
    cid="$(container_id "${service}")"
    if [[ -n "${cid}" ]]; then
      local running
      running="$(docker inspect --format '{{.State.Running}}' "${cid}")"
      if [[ "${running}" == "true" ]]; then
        if [[ "${health_required}" == "false" ]]; then
          log "${service} is running"
          return 0
        fi

        local health
        health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "${cid}")"
        if [[ "${health}" == "healthy" ]]; then
          log "${service} is healthy"
          return 0
        fi
        if [[ "${health}" == "unhealthy" ]]; then
          compose logs --tail=100 "${service}" >&2 || true
          fail "${service} is unhealthy"
        fi
      else
        local status
        status="$(docker inspect --format '{{.State.Status}}' "${cid}")"
        if [[ "${status}" == "exited" || "${status}" == "dead" ]]; then
          compose logs --tail=100 "${service}" >&2 || true
          fail "${service} is not running (${status})"
        fi
      fi
    fi
    sleep 2
  done

  compose logs --tail=100 "${service}" >&2 || true
  fail "timed out waiting for ${service} after ${WAIT_SECONDS}s"
}

for arg in "$@"; do
  case "${arg}" in
    --skip-git-update)
      SKIP_GIT_UPDATE="true"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      fail "unknown argument: ${arg}"
      ;;
  esac
done

require_command git
require_command docker
docker compose version >/dev/null 2>&1 || fail "Docker Compose v2 is required (docker compose)"

[[ -f "${COMPOSE_FILE}" ]] || fail "missing ${COMPOSE_FILE}"
[[ -f "${ENV_FILE}" ]] || fail "missing ${ENV_FILE}; create the production secret environment first"
[[ -f "${COMPOSE_ENV_FILE}" ]] || fail "missing ${COMPOSE_ENV_FILE}; it must at least define POSTGRES_PASSWORD"

if [[ "${SKIP_GIT_UPDATE}" == "false" ]]; then
  if [[ -n "$(git -C "${REPO_ROOT}" status --porcelain)" ]]; then
    fail "repository has uncommitted or untracked changes; commit/stash them before production deploy"
  fi

  log "updating repository to origin/main"
  git -C "${REPO_ROOT}" fetch origin main
  git -C "${REPO_ROOT}" switch main
  git -C "${REPO_ROOT}" pull --ff-only origin main
else
  log "skipping Git update; deploying current checkout"
fi

cd "${SCRIPT_DIR}"

log "validating production Compose configuration"
compose config --quiet

log "pulling published LifeTrace images"
compose pull

log "starting production stack"
compose up -d --remove-orphans

wait_for_migration
wait_for_service postgres true
wait_for_service lifetrace-cloud true
wait_for_service lifetrace-mail-worker true
wait_for_service lifetrace-execution-worker true
wait_for_service caddy false

log "production stack is healthy"
compose ps -a
log "deployed repository revision: $(git -C "${REPO_ROOT}" rev-parse HEAD)"
