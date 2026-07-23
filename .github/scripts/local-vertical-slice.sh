#!/usr/bin/env bash
set -euo pipefail

base_file="deploy/docker-compose.yml"
worktree_file="deploy/docker-compose.worktree.yml"
project_name="haze-sync-stage8-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
worktree_dir="$(mktemp -d)"
server_log="$(mktemp)"
config_json="$(mktemp)"
cli_human="$(mktemp)"
cli_json="$(mktemp)"
cli_plan_json="$(mktemp)"

export COMPOSE_PROJECT_NAME="$project_name"
export POSTGRES_DB="haze_sync_stage8"
export POSTGRES_USER="haze_sync_stage8"
export POSTGRES_PASSWORD="stage8-local-placeholder-password"
export POSTGRES_PORT="15434"
export HAZE_SYNC_HTTP_PORT="18082"
export HAZE_SYNC_WORKTREE_HOST_PATH="$worktree_dir"
export HAZE_SYNC_WORKTREE_ADAPTER_MODE="bidirectional"
export HAZE_SYNC_WORKTREE_BIND_READ_ONLY="false"

obsidian_token="stage8-obsidian-token-placeholder"
admin_token="stage8-admin-token-placeholder"
worktree_token="stage8-worktree-token-placeholder"
compose=(docker compose --project-name "$project_name" -f "$base_file" -f "$worktree_file")

cleanup() {
  "${compose[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
  rm -f "$server_log" "$config_json" "$cli_human" "$cli_json" "$cli_plan_json"
  if command -v sudo >/dev/null 2>&1; then
    sudo rm -rf "$worktree_dir"
  else
    rm -rf "$worktree_dir"
  fi
}
trap cleanup EXIT

chmod 0777 "$worktree_dir"
"${compose[@]}" config --quiet
"${compose[@]}" config --format json > "$config_json"
python3 - "$config_json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    config = json.load(handle)

if set(config.get("services", {})) != {"postgres", "server"}:
    raise SystemExit("Stage 8 topology must contain exactly PostgreSQL and Server")
server = config["services"]["server"]
if server.get("environment", {}).get("HAZE_SYNC_WORKTREE_ADAPTER_MODE") != "bidirectional":
    raise SystemExit("Stage 8 Worktree mode must be bidirectional")
volumes = server.get("volumes", [])
binds = [item for item in volumes if item.get("target") == "/var/lib/haze-sync/worktree"]
if len(binds) != 1 or binds[0].get("type") != "bind" or binds[0].get("read_only", False) is not False:
    raise SystemExit("Stage 8 Worktree bind must be one explicit read-write bind")
PY

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
  echo "Stage 8 PostgreSQL did not become ready." >&2
  exit 1
fi

for migration in migrations/*.sql; do
  "${compose[@]}" exec -T postgres \
    psql --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --set=ON_ERROR_STOP=1 \
    < "$migration"
done

hash_token() {
  printf '%s' "$1" | sha256sum | awk '{print $1}'
}

obsidian_hash="$(hash_token "$obsidian_token")"
admin_hash="$(hash_token "$admin_token")"
worktree_hash="$(hash_token "$worktree_token")"
"${compose[@]}" exec -T postgres psql \
  --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --set=ON_ERROR_STOP=1 \
  --set=obsidian_hash="sha256:${obsidian_hash}" \
  --set=admin_hash="sha256:${admin_hash}" \
  --set=worktree_hash="sha256:${worktree_hash}" <<'SQL'
insert into sync_adapters (adapter_id, display_name, role, token_hash, enabled)
values
  ('stage8-obsidian', 'Stage 8 Obsidian smoke', 'obsidian_plugin', :'obsidian_hash', true),
  ('stage8-admin', 'Stage 8 admin smoke', 'admin', :'admin_hash', true),
  ('worktree', 'Stage 8 Worktree host', 'worktree_adapter', :'worktree_hash', true);
SQL

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
    echo "Stage 8 Server exited before readiness." >&2
    exit 1
  fi
  sleep 1
done
if [ "$server_ready" -ne 1 ]; then
  "${compose[@]}" logs --no-color server >&2
  echo "Stage 8 Server did not become ready." >&2
  exit 1
fi

cargo build -p haze-sync-cli --locked
cli_bin="$PWD/target/debug/haze-sync"
export HAZE_SYNC_SERVER_URL="http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}"
export HAZE_SYNC_TOKEN_SOURCE="env:HAZE_SYNC_STAGE9_ADMIN_TOKEN"
export HAZE_SYNC_STAGE9_ADMIN_TOKEN="$admin_token"

"$cli_bin" preflight > "$cli_human"
grep -Fx 'preflight: ready' "$cli_human" >/dev/null
grep -F '[doctor]' "$cli_human" >/dev/null
grep -F '[adapters]' "$cli_human" >/dev/null
grep -F '[worktree]' "$cli_human" >/dev/null

"$cli_bin" --output json preflight > "$cli_json"
"$cli_bin" --output json recovery plan > "$cli_plan_json"
python3 - "$cli_json" "$cli_plan_json" <<'PYJSON'
import json
import sys

preflight_path, plan_path = sys.argv[1:]
with open(preflight_path, encoding="utf-8") as handle:
    preflight = json.load(handle)
with open(plan_path, encoding="utf-8") as handle:
    plan = json.load(handle)

if set(preflight) != {"schema", "command", "ok", "exit_code", "stdout", "stderr"}:
    raise SystemExit("Stage 9 preflight JSON fields are not the accepted v1 envelope")
if preflight["schema"] != "haze-sync.cli.output.v1" or preflight["command"] != "preflight":
    raise SystemExit("Stage 9 preflight JSON identity is invalid")
if not preflight["ok"] or preflight["exit_code"] != 0 or preflight["stderr"] != "":
    raise SystemExit("Stage 9 preflight JSON does not report success deterministically")
if "preflight: ready" not in preflight["stdout"]:
    raise SystemExit("Stage 9 preflight JSON does not carry the ready verdict")
if set(plan) != {"schema", "command", "ok", "exit_code", "stdout", "stderr"}:
    raise SystemExit("Stage 9 recovery plan JSON fields are not the accepted v1 envelope")
if plan["schema"] != "haze-sync.cli.output.v1" or plan["command"] != "recovery plan":
    raise SystemExit("Stage 9 recovery plan JSON identity is invalid")
if not plan["ok"] or plan["exit_code"] != 0 or plan["stderr"] != "":
    raise SystemExit("Stage 9 recovery plan JSON does not report success deterministically")
if "writes: none" not in plan["stdout"] or "requires explicit operator approval" not in plan["stdout"]:
    raise SystemExit("Stage 9 recovery plan is not an explicit no-write guarded dry run")
PYJSON

for plan in bootstrap recovery rollout; do
  "$cli_bin" "$plan" plan | grep -Fx 'writes: none' >/dev/null
done

for secret in "$admin_token" "$worktree_token" "$obsidian_token"; do
  if grep -F -- "$secret" "$cli_human" "$cli_json" "$cli_plan_json" >/dev/null; then
    echo "Stage 9 CLI output exposed a secret marker." >&2
    exit 1
  fi
done

HAZE_OBSIDIAN_SMOKE_SERVER_URL="http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}" \
HAZE_OBSIDIAN_SMOKE_AUTH_TOKEN="$obsidian_token" \
HAZE_OBSIDIAN_SMOKE_ADMIN_TOKEN="$admin_token" \
HAZE_OBSIDIAN_SMOKE_WORKTREE_PATH="$worktree_dir" \
npm run test:local-vertical-slice --workspace haze-obsidian-plugin

"${compose[@]}" exec -T server sh -ceu 'test "$(id -u)" = 10001'
"${compose[@]}" logs --no-color server > "$server_log"
for secret in "$POSTGRES_PASSWORD" "$obsidian_token" "$admin_token" "$worktree_token"; do
  if grep -F -- "$secret" "$server_log" >/dev/null; then
    echo "Stage 8 Server logs exposed a secret marker." >&2
    exit 1
  fi
done

"${compose[@]}" exec -T postgres psql \
  --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --tuples-only --no-align \
  --command="select count(*) from operation_log where path = 'stage8-local-vertical-slice.md'" \
  | grep -Eq '^[2-9][0-9]*$'

running_services="$("${compose[@]}" ps --services --status running | sort)"
if [ "$running_services" != $'postgres\nserver' ]; then
  printf 'Unexpected Stage 8 running services:\n%s\n' "$running_services" >&2
  exit 1
fi

curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/health" >/dev/null
curl --fail --silent --show-error "http://127.0.0.1:${HAZE_SYNC_HTTP_PORT}/ready" >/dev/null
