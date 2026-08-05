$ErrorActionPreference = "Stop"
docker compose -f deploy/cloud/docker-compose.local.yml --profile cloud up -d
