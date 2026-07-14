# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 API-P8 Sync-Once Race Fix
prompt_revision: verified by Orchestrator after CLEAN_NEEDS_FIX; focused authoritative-submit correction only

wave: W1
phase: FIX-SRV-API-P8-SUBMISSION-RACE

implementation_status: SELF_ACCEPT
fix_status: NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN_BUT_FUNCTIONALLY_INSUFFICIENT
known_failed_checks: []

candidate:
- current_code_bearing_sha: be2b1c16fa6c4919d446b76b1f15dca5767b2482
- review_report_blob: 04ba2d51a041b57c677a2a742d56719a181eaf67
- ci_run_id: 29321038276
- ci_run_number: 1938
- ci_conclusion: success
- db_capable: yes

blocking_finding:
- preliminary snapshot returns Busy/NotStarted/Cancelling/Shutdown without authoritative submit
- result may be stale and violate snapshot-to-submit race resolution

required_fix:
- snapshot-only precheck limited to Failed and manual/mode Unavailable
- all other states call submit_manual exactly once
- map typed Worktree submission result
- add deterministic stale-snapshot and single-submit tests
- preserve weak ownership, no-wait ticket semantics and API blobs

next_gate:
- FIX_COMPLETE plus green DB-capable exact-SHA CI -> final focused Server review
- CLEAN_ACCEPT after review -> CLI-P6A control-slot resolution
- Deployment remains blocked
