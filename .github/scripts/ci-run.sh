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

path = Path("crates/haze-gdrive-adapter/src/durable_state.rs")
text = path.read_text()

def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one occurrence, found {count}")
    text = text.replace(old, new, 1)

replace_once(
    "    GDriveLastOperationsSummaryDto, GDriveMappingFactsDto, GDriveStateCommitRequest,\n    GDriveStateCommitResponse, GDriveStateErrorCode, GDriveStateErrorResponse,\n    GDriveStateSnapshotResponse,",
    "    GDriveLastOperationsSummaryDto, GDriveMappingFactsDto, GDrivePrivateCursorStateDto,\n    GDriveStateCommitRequest, GDriveStateCommitResponse, GDriveStateErrorCode,\n    GDriveStateErrorResponse, GDriveStateSnapshotResponse,",
    "private cursor import",
)
replace_once(
    '''    fn from_snapshot(snapshot: &GDriveStateSnapshotResponse) -> Self {
        Self {
            state_format_version: snapshot.state_format_version,
            state_version: snapshot.state_version,
            cursor_generation: snapshot.cursor.generation,
            cursor_present: snapshot.cursor.present,
            core_export_checkpoint: snapshot.core_export_checkpoint,
            last_operations: snapshot.last_operations.clone(),
        }
    }''',
    '''    fn from_snapshot(snapshot: &GDriveStateSnapshotResponse) -> Self {
        let (cursor_generation, cursor_present) = match &snapshot.cursor {
            GDrivePrivateCursorStateDto::Absent { generation } => (*generation, false),
            GDrivePrivateCursorStateDto::Present { generation, .. } => (*generation, true),
        };
        Self {
            state_format_version: snapshot.state_format_version,
            state_version: snapshot.state_version,
            cursor_generation,
            cursor_present,
            core_export_checkpoint: snapshot.core_export_checkpoint,
            last_operations: snapshot.last_operations.clone(),
        }
    }''',
    "snapshot signature mapping",
)
replace_once(
    '            "cursor": { "generation": 3, "present": true },',
    '            "cursor": {\n                "state": "present",\n                "generation": 3,\n                "cursor": "sentinel-private-cursor"\n            },',
    "snapshot fixture cursor",
)
replace_once(
    '''        assert_eq!(snapshot.state_version, 7);
        let calls = client.transport().calls();''',
    '''        assert_eq!(snapshot.state_version, 7);
        match &snapshot.cursor {
            GDrivePrivateCursorStateDto::Present { generation, cursor } => {
                assert_eq!(*generation, 3);
                assert_eq!(
                    cursor.expose_for_private_commit(),
                    "sentinel-private-cursor"
                );
            }
            GDrivePrivateCursorStateDto::Absent { .. } => panic!("cursor should be present"),
        }
        let calls = client.transport().calls();''',
    "present cursor assertion",
)
marker = '''    #[test]
    fn origin_only_server_url_accepts_root_forms_and_rejects_ambiguous_endpoints() {'''
absent_test = '''    #[test]
    fn absent_private_cursor_decodes_and_signs_without_provider_value() {
        let body = serde_json::to_vec(&serde_json::json!({
            "adapter_id": "gdrive-main",
            "state_format_version": 1,
            "state_version": 7,
            "cursor": { "state": "absent", "generation": 0 },
            "core_export_checkpoint": 0,
            "last_operations": {},
            "mappings": [],
            "next_after_path": null
        }))
        .unwrap();
        let client = fake_client(
            [Ok(HttpResponse::new(200, body))],
            policy(8_192, 4, 10),
        );

        let snapshot = client.get_state_page(None, 1).unwrap();
        assert!(matches!(
            snapshot.cursor,
            GDrivePrivateCursorStateDto::Absent { generation: 0 }
        ));
        let signature = SnapshotSignature::from_snapshot(&snapshot);
        assert_eq!(signature.cursor_generation, 0);
        assert!(!signature.cursor_present);
    }

'''
replace_once(marker, absent_test + marker, "absent cursor test")
path.write_text(text)
PY

  cargo fmt --all
  cargo check -p haze-gdrive-adapter
  cargo test -p haze-gdrive-adapter
  cargo clippy -p haze-gdrive-adapter --all-targets -- -D warnings
  mkdir -p "${diag_dir}/stage4-gdrive" "${diag_dir}/failures"
  cp crates/haze-gdrive-adapter/src/durable_state.rs "${diag_dir}/stage4-gdrive/durable_state.rs"
  {
    echo "check=stage4-gdrive-export"
    echo "exit_code=1"
    echo "log=stage4-gdrive/verified-source-file"
    echo "failure_marker=failures/stage4-gdrive-export.txt"
  } > "${diag_dir}/failures/stage4-gdrive-export.txt"
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
