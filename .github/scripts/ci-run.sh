#!/usr/bin/env bash
set -uo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <check-name> <command> [args...]" >&2
  exit 2
fi

check_name="$1"
shift

diag_dir="${CI_DIAGNOSTICS_DIR:-ci-diagnostics}"
tmp_dir="${RUNNER_TEMP:-/tmp}/haze-ci-logs"
tmp_log="${tmp_dir}/${check_name}.log"

mkdir -p "$tmp_dir"

{
  echo "## check"
  echo "$check_name"
  echo
  echo "## command"
  printf '%q ' "$@"
  echo
  echo
  echo "## output"
} > "$tmp_log"

set +e
"$@" 2>&1 | tee -a "$tmp_log"
status=${PIPESTATUS[0]}
set -e

if [ "$status" -ne 0 ]; then
  mkdir -p "${diag_dir}/logs" "${diag_dir}/failures"

  cp "$tmp_log" "${diag_dir}/logs/${check_name}.log"

  {
    echo "check=${check_name}"
    echo "exit_code=${status}"
    echo "log=logs/${check_name}.log"
    echo "failure_marker=failures/${check_name}.txt"
    printf 'command='
    printf '%q ' "$@"
    echo
  } > "${diag_dir}/failures/${check_name}.txt"

  echo "::error title=${check_name} failed::exit code ${status}; see diagnostics artifact"
fi

# Return success so later checks run. ci-finalize.sh fails the job if any
# failure markers were written.
exit 0
