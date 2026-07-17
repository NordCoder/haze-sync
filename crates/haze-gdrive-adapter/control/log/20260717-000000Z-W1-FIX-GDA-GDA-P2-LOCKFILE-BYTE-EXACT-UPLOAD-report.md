REPORT_TYPE: FIX
STATUS: FIX_BLOCKED_BY_TOOLING
phase_id: FIX-GDA-GDA-P2-LOCKFILE-BYTE-EXACT-UPLOAD
chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P2 Lockfile Byte-Exact Upload

INPUTS:
- artifact: 8382573523
- corrupt_sha: 8aa7616152bd24711bb6a494170885c4d4a5bbe1
- corrupt_lock_blob: 256a4fe0c39f2a2d40511aec1f74f3ebb064428a

EXACT_FILE:
- bytes: 61847
- package_entries: 257
- final_newline: yes
- generated_blob: 882be8e8ce61ac4c77e8bdaec45d1cbaa030aa86
- repository_blob: 882be8e8ce61ac4c77e8bdaec45d1cbaa030aa86

CODE_COMMIT:
- sha: 746dc8790643e13e85553ff94f6b124a5c686127
- parent: adef1a958420e85575ac01c5ec4b449c95b55aaa
- changed_path: Cargo.lock only
- ref_update: fast-forward

NOTE:
- commit adef1a958420e85575ac01c5ec4b449c95b55aaa produced wrong blob 851105e5f7adff2e5b1d7e4c3a800db885872849 and was not accepted
- the forward commit above attaches the exact Cargo-generated blob
- no checksum field was individually edited

CI:
- exact_sha: 746dc8790643e13e85553ff94f6b124a5c686127
- verified_run: none
- reason: repeated upstream 502 responses from GitHub commit, PR, status and Actions metadata calls
- no run identifier or conclusion was inferred

UNCHANGED:
- source, tests, manifests, workflows, sibling files, control/state.md

FINAL_VERDICT:
FIX_BLOCKED_BY_TOOLING. The exact generated lockfile is committed, but exact-SHA Component CI is not verifiable through the current GitHub connector. FIX_COMPLETE, CLEAN_ACCEPT and GDA-GDA-P4 authorization are not claimed.
