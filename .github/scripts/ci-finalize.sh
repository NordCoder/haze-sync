#!/usr/bin/env bash
set -euo pipefail

diag_dir="${CI_DIAGNOSTICS_DIR:-ci-diagnostics}"
failures_dir="${diag_dir}/failures"

if [ ! -d "$failures_dir" ] || [ -z "$(find "$failures_dir" -type f -name '*.txt' -print -quit)" ]; then
  echo "No CI check failures collected."
  exit 0
fi

mkdir -p "$diag_dir"

component="${HAZE_COMPONENT:-unknown}"
component_branch="${HAZE_COMPONENT_BRANCH:-unknown}"
workflow_slug="${HAZE_WORKFLOW_SLUG:-component-ci}"
created_at_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

summary_file="${diag_dir}/summary.md"
manifest_file="${diag_dir}/manifest.json"

{
  echo "# CI diagnostics"
  echo
  echo "schema: haze-ci-diagnostics-v1"
  echo "component: ${component}"
  echo "component_branch: ${component_branch}"
  echo "workflow: ${GITHUB_WORKFLOW:-unknown}"
  echo "workflow_slug: ${workflow_slug}"
  echo "run_id: ${GITHUB_RUN_ID:-unknown}"
  echo "run_attempt: ${GITHUB_RUN_ATTEMPT:-unknown}"
  echo "run_number: ${GITHUB_RUN_NUMBER:-unknown}"
  echo "event: ${GITHUB_EVENT_NAME:-unknown}"
  echo "head_sha: ${GITHUB_SHA:-unknown}"
  echo "created_at_utc: ${created_at_utc}"
  echo "retention_days: 1"
  echo
  echo "## Failed checks"
  echo

  for failure in "$failures_dir"/*.txt; do
    check="$(grep '^check=' "$failure" | cut -d= -f2-)"
    exit_code="$(grep '^exit_code=' "$failure" | cut -d= -f2-)"
    log="$(grep '^log=' "$failure" | cut -d= -f2-)"
    marker="$(grep '^failure_marker=' "$failure" | cut -d= -f2-)"
    command="$(grep '^command=' "$failure" | cut -d= -f2-)"

    echo "### ${check}"
    echo
    echo "- exit_code: ${exit_code}"
    echo "- log: ${log}"
    echo "- failure_marker: ${marker}"
    echo
    echo '```bash'
    echo "$command"
    echo '```'
    echo
  done

  echo "## Fixer rule"
  echo
  echo "Fix the minimum cause of failure inside the assigned component scope."
  echo
  echo "If these diagnostics require another component, a contract change, or CI harness work, report the appropriate blocked status instead of silently broadening scope."
} > "$summary_file"

{
  echo "{"
  echo "  \"schema\": \"haze-ci-diagnostics-v1\"," 
  echo "  \"component\": \"${component}\"," 
  echo "  \"component_branch\": \"${component_branch}\"," 
  echo "  \"workflow\": \"${GITHUB_WORKFLOW:-unknown}\"," 
  echo "  \"workflow_slug\": \"${workflow_slug}\"," 
  echo "  \"run_id\": \"${GITHUB_RUN_ID:-unknown}\"," 
  echo "  \"run_attempt\": \"${GITHUB_RUN_ATTEMPT:-unknown}\"," 
  echo "  \"run_number\": \"${GITHUB_RUN_NUMBER:-unknown}\"," 
  echo "  \"event\": \"${GITHUB_EVENT_NAME:-unknown}\"," 
  echo "  \"head_sha\": \"${GITHUB_SHA:-unknown}\"," 
  echo "  \"created_at_utc\": \"${created_at_utc}\"," 
  echo "  \"retention_days\": 1,"
  echo "  \"failed_checks\": ["

  first=1
  for failure in "$failures_dir"/*.txt; do
    check="$(grep '^check=' "$failure" | cut -d= -f2-)"
    exit_code="$(grep '^exit_code=' "$failure" | cut -d= -f2-)"
    log="$(grep '^log=' "$failure" | cut -d= -f2-)"
    marker="$(grep '^failure_marker=' "$failure" | cut -d= -f2-)"

    if [ "$first" -eq 0 ]; then
      echo ","
    fi
    first=0

    printf '    {\n'
    printf '      "check": "%s",\n' "$check"
    printf '      "exit_code": %s,\n' "$exit_code"
    printf '      "log": "%s",\n' "$log"
    printf '      "failure_marker": "%s"\n' "$marker"
    printf '    }'
  done

  echo
  echo "  ]"
  echo "}"
} > "$manifest_file"

echo "One or more CI checks failed. See ${summary_file}."
exit 1
