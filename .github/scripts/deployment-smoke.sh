#!/usr/bin/env bash
set -euo pipefail

base_file="deploy/docker-compose.yml"
worktree_file="deploy/docker-compose.worktree.yml"
project_name="haze-sync-stage5-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
worktree_dir="$(mktemp -d)"
base_config_json="$(mktemp)"
worktree_config_json="$(mktemp)"
server_log="$(mktemp)"

export COMPOSE_PROJECT_NAME="$project_name"
export POSTGRES_DB="haze_sync_stage5"
export POSTGRES_USER="haze_sync_stage5"
export POSTGRES_PASSWORD="stage5-local-placeholder-password"
export POSTGRES_PORT="15432"
export HAZE_SYNC_HTTP_PORT="18081"
export HAZE_SYNC_WORKTREE_HOST_PATH="$worktree_dir"
export HAZE_SYNC_WORKTREE_ADAPTER_MODE="disabled"
export HAZE_SYNC_WORKTREE_BIND_READ_ONLY="true"

compose=(docker compose --project-name "$project_name" -f "$base_file")
compose_worktree=(docker compose --project-name "$project_name" -f "$base_file" -f "$worktree_file")

cleanup() {
  "${compose[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$worktree_dir" "$base_config_json" "$worktree_config_json" "$server_log"
}
trap cleanup EXIT

"${compose[@]}" config --quiet
"${compose_worktree[@]}" config --quiet
"${compose[@]}" config --format json > "$base_config_json"
"${compose_worktree[@]}" config --format json > "$worktree_config_json"

python3 - "$base_config_json" "$worktree_config_json" <<'PY'
import json
import sys

base_path, worktree_path = sys.argv[1:]
with open(base_path, encoding="utf-8") as handle:
    base = json.load(handle)
with open(worktree_path, encoding="utf-8") as handle:
    worktree = json.load(handle)

expected_services = {"postgres", "server"}
if set(base.get("services", {})) != expected_services:
    raise SystemExit("base Compose must contain exactly postgres and server")
if set(worktree.get("services", {})) != expected_services:
    raise SystemExit("Worktree override must not add services")

server_environment = base["services"]["server"].get("environment", {})
for key in (
    "HAZE_SYNC_WORKTREE_ADAPTER_MODE",
    "HAZE_GDRIVE_ADAPTER_MODE",
    "HAZE_OBSIDIAN_ADAPTER_MODE",
):
    if server_environment.get(key) != "disabled":
        raise SystemExit(f"{key} must remain disabled in base Compose")

volumes = worktree["services"]["server"].get("volumes", [])
binds = [volume for volume in volumes if volume.get("target") == "/var/lib/haze-sync/worktree"]
if len(binds) != 1:
    raise SystemExit("Worktree override must define exactly one Worktree bind")
bind = binds[0]
if bind.get("type") != "bind" or bind.get("read_only") is not True:
    raise SystemExit("Worktree bind must remain read-only by default")
PY

docker run --rm \
  --volume "$PWD/deploy/reverse-proxy/Caddyfile:/etc/caddy/Caddyfile:ro" \
  caddy:2.10-alpine \
  caddy validate --config /etc/caddy/Caddyfile

"${compose[@]}" build server
"${compose[@]}" up -d postgres

postgres_ready=0
for _ in $(seq 1 60); do
  if "${compose[@]}" exec -T postgres \
    pg_isready --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" >/dev/null 2>&1; then
    postgres_ready=1
    break
  fi
  sleep 1
done
if [ "$postgres_ready" -ne 1 ]; then
  "${compose[@]}" logs --no-color postgres >&2
  echo "Compose PostgreSQL did not become ready." >&2
  exit 1
fi

for migration in migrations/*.sql; do
  "${compose[@]}" exec -T postgres \
    psql --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --set=ON_ERROR_STOP=1 \
    < "$migration"
done

"${compose[@]}" up -d server

server_ready=0
for _ in $(seq 1 90); do
  if curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/health" >/dev/null \
    && curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/ready" >/dev/null; then
    server_ready=1
    break
  fi
  if ! "${compose[@]}" ps --services --status running | grep -Fx server >/dev/null; then
    "${compose[@]}" logs --no-color server >&2
    echo "Compose Server exited before readiness." >&2
    exit 1
  fi
  sleep 1
done
if [ "$server_ready" -ne 1 ]; then
  "${compose[@]}" logs --no-color server >&2
  echo "Compose Server did not become ready." >&2
  exit 1
fi

"${compose[@]}" exec -T server sh -ceu 'test "$(id -u)" = 10001'

"${compose[@]}" logs --no-color server > "$server_log"
if grep -F -- "$POSTGRES_PASSWORD" "$server_log" >/dev/null; then
  echo "Compose Server logs exposed the local database password." >&2
  exit 1
fi

running_services="$("${compose[@]}" ps --services --status running | sort)"
if [ "$running_services" != $'postgres\nserver' ]; then
  printf 'Unexpected running services:\n%s\n' "$running_services" >&2
  exit 1
fi

curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/health" >/dev/null
curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/ready" >/dev/null
