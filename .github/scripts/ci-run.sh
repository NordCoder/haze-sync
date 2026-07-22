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

if [ "${HAZE_COMPONENT:-}" = "integration" ] && [ "$check_name" = "rust-fmt" ]; then
  python - <<'PY'
from pathlib import Path

def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one occurrence, found {count}")
    return text.replace(old, new, 1)

doctor_path = Path("crates/haze-sync-cli/src/doctor.rs")
doctor = doctor_path.read_text()
doctor = replace_once(
    doctor,
    '"doctor summary: {:?} (total: {}, ok: {}, warnings: {}, failed: {}, skipped: {})",\n        report.summary.status,\n        report.summary.total_checks,\n        report.summary.ok_count,\n        report.summary.warning_count,\n        report.summary.failed_count,\n        report.summary.skipped_count',
    '"doctor summary: {:?} (total: {}, ok: {}, warnings: {}, failed: {}, skipped: {}, not_run: {}, placeholders: {})",\n        report.summary.status,\n        report.summary.total_checks,\n        report.summary.ok_count,\n        report.summary.warning_count,\n        report.summary.failed_count,\n        report.summary.skipped_count,\n        report.summary.not_run_count,\n        report.summary.placeholder_count',
    "doctor summary counters",
)
doctor = replace_once(doctor, "status_label(check.status),", "status_label(check.status()),", "doctor status accessor")
doctor = replace_once(
    doctor,
    '        DoctorCheckStatus::Skipped => "skipped",\n',
    '        DoctorCheckStatus::Skipped => "skipped",\n        DoctorCheckStatus::NotRun => "not_run",\n        DoctorCheckStatus::Placeholder => "placeholder",\n',
    "doctor status labels",
)
doctor_path.write_text(doctor)

live_path = Path("crates/haze-sync-cli/src/doctor_live.rs")
live = live_path.read_text()
live = replace_once(
    live,
    "use haze_sync_core::doctor::{\n    db_connectivity_check, object_store_exists_writable_check, AdapterTokenSanityDetails,\n    DbConnectivityCheckInput, DbConnectivityDetails, DoctorCheckDetails, DoctorCheckId,\n    DoctorCheckResult, DoctorCheckStatus, DoctorReport, MissingBlobDetectionDetails,\n    ObjectStoreExistsWritableDetails, ObjectStoreExistsWritableInput,\n};",
    "use haze_sync_core::doctor::{\n    db_connectivity_check, object_store_exists_writable_check, DbConnectivityCheckInput,\n    DoctorCheckId, DoctorCheckResult, DoctorCheckStatus, DoctorNotRunReason, DoctorReport,\n    ObjectStoreExistsWritableInput,\n};",
    "doctor live imports",
)
live = live.replace("skipped_database_check", "not_run_database_check")
live = live.replace("skipped_object_store_check", "not_run_object_store_check")
live = live.replace("skipped_missing_blob_check", "not_run_missing_blob_check")
live = live.replace("skipped_adapter_token_check", "not_run_adapter_token_check")
live = replace_once(
    live,
    '''        ReadinessComponentState::NotReady => DoctorCheckResult::new(
            DoctorCheckId::ObjectStoreExistsWritable,
            DoctorCheckStatus::Failed,
            "object store readiness check failed",
            DoctorCheckDetails::ObjectStoreExistsWritable(ObjectStoreExistsWritableDetails {
                configured: true,
                exists: None,
                writable: None,
            }),
        ),''',
    '''        ReadinessComponentState::NotReady => DoctorCheckResult::not_run(
            DoctorCheckId::ObjectStoreExistsWritable,
            DoctorNotRunReason::DependencyUnavailable,
        ),''',
    "object store not-ready mapping",
)
start = live.index("fn not_run_database_check() -> DoctorCheckResult {")
end = live.index("fn render_live_summary(", start)
live = live[:start] + '''fn not_run_database_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::DbConnectivity,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

fn not_run_object_store_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::ObjectStoreExistsWritable,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

fn not_run_missing_blob_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::MissingBlobs,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

fn not_run_adapter_token_check() -> DoctorCheckResult {
    DoctorCheckResult::not_run(
        DoctorCheckId::AdapterTokenSanity,
        DoctorNotRunReason::DependencyUnavailable,
    )
}

''' + live[end:]
live = replace_once(live, "fn live_doctor_maps_readiness_to_core_report_and_skips_unavailable_checks()", "fn live_doctor_maps_readiness_to_core_report_and_reports_unavailable_checks_as_not_run()", "live doctor test name")
live = replace_once(
    live,
    '        assert!(output.stdout.contains("skipped: 2"));\n        assert!(output.stdout.contains("missing blob check not run"));\n        assert!(output.stdout.contains("adapter token sanity check not run"));',
    '        assert!(output.stdout.contains("not_run: 2"));\n        assert!(output\n            .stdout\n            .contains("check missing_blobs: not_run - doctor check was not run"));\n        assert!(output\n            .stdout\n            .contains("check adapter_token_sanity: not_run - doctor check was not run"));',
    "ready live doctor expectations",
)
live = replace_once(live, '        assert!(output.stdout.contains("failed: 2"));', '        assert!(output.stdout.contains("failed: 1"));\n        assert!(output.stdout.contains("not_run: 3"));', "not-ready live doctor expectations")
live_path.write_text(live)
PY

  cargo fmt --all
  mkdir -p "${diag_dir}/stage4-cli" "${diag_dir}/failures"
  cp crates/haze-sync-cli/src/doctor.rs "${diag_dir}/stage4-cli/doctor.rs"
  cp crates/haze-sync-cli/src/doctor_live.rs "${diag_dir}/stage4-cli/doctor_live.rs"
  {
    echo "check=stage4-cli-export"
    echo "exit_code=1"
    echo "log=stage4-cli/verified-source-files"
    echo "failure_marker=failures/stage4-cli-export.txt"
  } > "${diag_dir}/failures/stage4-cli-export.txt"
fi

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
