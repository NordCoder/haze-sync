# W1-FIX-GDA-GDA-P2-LOCKFILE-BYTE-EXACT-UPLOAD

Name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P2 Lockfile Byte-Exact Upload`

Repository: `NordCoder/haze-sync`
Branch: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P2-LOCKFILE-BYTE-EXACT-UPLOAD`

Do not begin another product phase.

## Verified coordinates

- corrupt product SHA: `8aa7616152bd24711bb6a494170885c4d4a5bbe1`
- corrupt `Cargo.lock` blob: `256a4fe0c39f2a2d40511aec1f74f3ebb064428a`
- failing CI run: `29519686947`, number `2058`
- failing artifact: `8384159071`
- prior recovery report blob: `3adaea959f1d63bbff38af1e5ed2717e67ec94cf`

## Exact generated file

Artifact `8382573523` from run `29515680307` contains the complete Cargo-generated lockfile in `logs/cargo-test.log` between:

`BEGIN_GENERATED_LOCKFILE`

and

`END_GENERATED_LOCKFILE`

Exclude the markers and preserve the final newline. Verified identity:

- bytes: `61847`
- package entries: `257`
- Git blob SHA: `882be8e8ce61ac4c77e8bdaec45d1cbaa030aa86`

## Required work

1. Download artifact `8382573523`.
2. Extract the full text between the markers, preserving the final newline.
3. Verify all three identity values above.
4. Replace repository `Cargo.lock` with the complete extracted UTF-8 text using GitHub `update_file`.
5. Do not edit or patch individual checksum fields.
6. Verify the resulting repository blob is exactly `882be8e8ce61ac4c77e8bdaec45d1cbaa030aa86`.
7. Record the real commit SHA returned by GitHub. Do not use CI skip.
8. Obtain full exact-SHA Component CI green and verify the run resolves to that commit.
9. Change no source, manifest, workflow or sibling file.

Write only `crates/haze-gdrive-adapter/control/report.md` with `REPORT_TYPE: FIX`, this phase and chat name, and status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_TOOLING`, or `FIX_BLOCKED_BY_CONTRACT`.

Record extraction checks, final repository blob, real code-bearing SHA and real CI. Do not claim CLEAN_ACCEPT.
