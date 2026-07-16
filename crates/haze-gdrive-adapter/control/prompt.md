# W1-FIX-GDA-GDA-P2-LOCKFILE-RECOVERY

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P2 Lockfile Recovery`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P2-LOCKFILE-RECOVERY`

This is a control-integrity and lockfile recovery fixer for `GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT`. Do not begin the long-running runtime.

## Corrected authoritative coordinates

The previous fixer report claimed a candidate and CI run that do not exist. They are not accepted evidence.

- actual last product commit: `8aa7616152bd24711bb6a494170885c4d4a5bbe1`;
- actual committed Cargo.lock blob: `256a4fe0c39f2a2d40511aec1f74f3ebb064428a`;
- actual failing Component CI: run `29519686947`, number `2058`;
- diagnostics artifact: `8384159071`;
- blocked clean-review report blob: `e907d38bb27d422715061b30647bdf34577f7fc3`;
- prior inaccurate fixer report blob: `ae20c8c1bc6bbe4f4f03c000a1be46d99d99bacf`.

Verified failure: the committed lockfile contains registry checksums that differ from Cargo registry metadata. Commit `8aa761...` manually changed checksum text. This is invalid.

## Required protocol

1. Read artifact `8384159071` first: `summary.md`, `manifest.json`, every failure marker, and every failed-check log. Use raw job logs only if the artifact is unavailable or incomplete.
2. Regenerate the complete `Cargo.lock` using Cargo from the final workspace manifests. Never manually edit, copy, guess or patch registry checksum fields.
3. Commit the exact Cargo-generated lockfile. Do not change unrelated dependency intent.
4. Preserve the currently reachable non-lock fixes unless CI proves a focused defect:
   - bounded whole-request deadline and timeout classification;
   - origin-only Server endpoint and exact accepted `/v1` routes;
   - AdapterConfig/AdapterRuntime provider-root and endpoint redaction;
   - byte-identical accepted API owner files;
   - mode gating, pagination, strict decoding and no automatic POST retry.
5. Obtain a real, resolvable code-bearing commit SHA and full exact-SHA Component CI with fmt/check/test/clippy and diagnostics finalization green. Record only coordinates verified through GitHub.
6. Verify the committed `Cargo.lock` blob equals the exact generated file used by the green build. A `--locked` check is preferred where available.
7. If the available execution environment cannot generate a trustworthy lockfile through Cargo, report `FIX_BLOCKED_BY_TOOLING`. Do not fabricate a SHA, blob or CI run.

## Allowed scope

- `Cargo.lock`;
- GDrive manifests only if Cargo proves a minimal manifest correction is necessary;
- focused GDrive tests only if the artifact proves another regression;
- control report.

## Forbidden

- manual registry checksum editing;
- invented or unverified commit/blob/run identifiers;
- API/Server/Storage/Core semantic changes;
- runtime loop, provider synchronization, direct DB, status-control, CLI or Deployment work;
- workflow changes, test weakening, live credentials or external-network CI;
- merge, rebase, force-push or PR draft-state changes.

Create product/lockfile changes without CI skip. Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-GDA-GDA-P2-LOCKFILE-RECOVERY`;
- `chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P2 Lockfile Recovery`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_TOOLING`, or `FIX_BLOCKED_BY_CONTRACT`.

Record artifact files read, exact Cargo generation method, final lockfile blob, real final code-bearing SHA and real exact-SHA CI. Do not claim CLEAN_ACCEPT; a repeat HTTP/security review follows.