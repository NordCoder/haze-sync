#!/usr/bin/env bash
set -euo pipefail

base_file="deploy/docker-compose.yml"
worktree_file="deploy/docker-compose.worktree.yml"
gdrive_file="deploy/docker-compose.gdrive.yml"
project_name="haze-sync-stage7-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
worktree_dir="$(mktemp -d)"
secret_dir="$(mktemp -d)"
base_config_json="$(mktemp)"
worktree_config_json="$(mktemp)"
gdrive_config_json="$(mktemp)"
server_log="$(mktemp)"
gdrive_log="$(mktemp)"

chmod 700 "$secret_dir"
printf '%s' 'stage7-adapter-token-placeholder' > "$secret_dir/adapter-token"
printf '%s\n' 'stage7-disabled-oauth-placeholder' > "$secret_dir/oauth"
chmod 600 "$secret_dir/adapter-token" "$secret_dir/oauth"

export COMPOSE_PROJECT_NAME="$project_name"
export POSTGRES_DB="haze_sync_stage7"
export POSTGRES_USER="haze_sync_stage7"
export POSTGRES_PASSWORD="stage7-local-placeholder-password"
export POSTGRES_PORT="15432"
export HAZE_SYNC_HTTP_PORT="18081"
export HAZE_SYNC_WORKTREE_HOST_PATH="$worktree_dir"
export HAZE_SYNC_WORKTREE_ADAPTER_MODE="disabled"
export HAZE_SYNC_WORKTREE_BIND_READ_ONLY="true"
export HAZE_GDRIVE_ADAPTER_ID="stage7-gdrive-adapter"
export HAZE_GDRIVE_ADAPTER_TOKEN_SECRET_FILE="$secret_dir/adapter-token"
export HAZE_GDRIVE_OAUTH_SECRET_FILE="$secret_dir/oauth"
export HAZE_GDRIVE_ROOT_FOLDER_ID="stage7-disabled-root"
export HAZE_GDRIVE_MODE="disabled"
export HAZE_GDRIVE_POLL_INTERVAL_SECONDS="1"
export HAZE_GDRIVE_FULL_SCAN_INTERVAL_SECONDS="2"
export HAZE_GDRIVE_MAX_DELETES_PER_RUN="0"
export HAZE_GDRIVE_MAX_DELETE_RATIO_PERCENT="0"

compose=(docker compose --project-name "$project_name" -f "$base_file")
compose_worktree=(docker compose --project-name "$project_name" -f "$base_file" -f "$worktree_file")
compose_gdrive=(docker compose --project-name "$project_name" --profile gdrive -f "$base_file" -f "$gdrive_file")

cleanup() {
  "${compose_gdrive[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
  "${compose[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$worktree_dir" "$secret_dir" \
    "$base_config_json" "$worktree_config_json" "$gdrive_config_json" \
    "$server_log" "$gdrive_log"
}
trap cleanup EXIT

"${compose[@]}" config --quiet
"${compose_worktree[@]}" config --quiet
"${compose_gdrive[@]}" config --quiet
"${compose[@]}" config --format json > "$base_config_json"
"${compose_worktree[@]}" config --format json > "$worktree_config_json"
"${compose_gdrive[@]}" config --format json > "$gdrive_config_json"

python3 - "$base_config_json" "$worktree_config_json" "$gdrive_config_json" <<'PY'
import json
import sys

base_path, worktree_path, gdrive_path = sys.argv[1:]
with open(base_path, encoding="utf-8") as handle:
    base = json.load(handle)
with open(worktree_path, encoding="utf-8") as handle:
    worktree = json.load(handle)
with open(gdrive_path, encoding="utf-8") as handle:
    gdrive = json.load(handle)

base_services = {"postgres", "server"}
if set(base.get("services", {})) != base_services:
    raise SystemExit("base Compose must contain exactly postgres and server")
if set(worktree.get("services", {})) != base_services:
    raise SystemExit("Worktree override must not add services")
if set(gdrive.get("services", {})) != {"postgres", "server", "gdrive"}:
    raise SystemExit("GDrive override must add exactly one gdrive service")

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

gdrive_service = gdrive["services"]["gdrive"]
if gdrive_service.get("environment", {}).get("HAZE_GDRIVE_MODE") != "disabled":
    raise SystemExit("GDrive service must remain disabled by default in Stage 7 smoke")
if gdrive_service.get("read_only") is not True:
    raise SystemExit("GDrive container root filesystem must be read-only")
if set(gdrive_service.get("cap_drop", [])) != {"ALL"}:
    raise SystemExit("GDrive service must drop all Linux capabilities")
if "no-new-privileges:true" not in gdrive_service.get("security_opt", []):
    raise SystemExit("GDrive service must enable no-new-privileges")
PY

docker run --rm \
  --volume "$PWD/deploy/reverse-proxy/Caddyfile:/etc/caddy/Caddyfile:ro" \
  caddy:2.10-alpine \
  caddy validate --config /etc/caddy/Caddyfile

"${compose_gdrive[@]}" build server gdrive
"${compose_gdrive[@]}" up -d postgres

postgres_ready=0
for _ in $(seq 1 60); do
  if "${compose_gdrive[@]}" exec -T postgres \
    pg_isready --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" >/dev/null 2>&1; then
    postgres_ready=1
    break
  fi
  sleep 1
done
if [ "$postgres_ready" -ne 1 ]; then
  "${compose_gdrive[@]}" logs --no-color postgres >&2
  echo "Compose PostgreSQL did not become ready." >&2
  exit 1
fi

for migration in migrations/*.sql; do
  "${compose_gdrive[@]}" exec -T postgres \
    psql --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --set=ON_ERROR_STOP=1 \
    < "$migration"
done

"${compose_gdrive[@]}" up -d server gdrive

server_ready=0
for _ in $(seq 1 90); do
  if curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/health" >/dev/null \
    && curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/ready" >/dev/null; then
    server_ready=1
    break
  fi
  if ! "${compose_gdrive[@]}" ps --services --status running | grep -Fx server >/dev/null; then
    "${compose_gdrive[@]}" logs --no-color server >&2
    echo "Compose Server exited before readiness." >&2
    exit 1
  fi
  sleep 1
done
if [ "$server_ready" -ne 1 ]; then
  "${compose_gdrive[@]}" logs --no-color server >&2
  echo "Compose Server did not become ready." >&2
  exit 1
fi

for _ in $(seq 1 30); do
  if "${compose_gdrive[@]}" ps --services --status running | grep -Fx gdrive >/dev/null; then
    break
  fi
  sleep 1
done
if ! "${compose_gdrive[@]}" ps --services --status running | grep -Fx gdrive >/dev/null; then
  "${compose_gdrive[@]}" logs --no-color gdrive >&2
  echo "Compose GDrive service did not remain running in disabled mode." >&2
  exit 1
fi

"${compose_gdrive[@]}" exec -T server sh -ceu 'test "$(id -u)" = 10001'
"${compose_gdrive[@]}" exec -T gdrive sh -ceu 'test "$(id -u)" = 10002'

"${compose_gdrive[@]}" logs --no-color server > "$server_log"
"${compose_gdrive[@]}" logs --no-color gdrive > "$gdrive_log"
for secret in "$POSTGRES_PASSWORD" 'stage7-adapter-token-placeholder' 'stage7-disabled-oauth-placeholder'; do
  if grep -F -- "$secret" "$server_log" "$gdrive_log" >/dev/null; then
    echo "Compose logs exposed a Stage 7 secret marker." >&2
    exit 1
  fi
done

if ! grep -F -- 'mode=disabled' "$gdrive_log" >/dev/null; then
  echo "GDrive disabled-mode startup evidence is missing." >&2
  exit 1
fi

running_services="$("${compose_gdrive[@]}" ps --services --status running | sort)"
if [ "$running_services" != $'gdrive\npostgres\nserver' ]; then
  printf 'Unexpected running services:\n%s\n' "$running_services" >&2
  exit 1
fi

curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/health" >/dev/null
curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/ready" >/dev/null
