# Control State

component: gdrive-adapter
repository: NordCoder/haze-sync
branch: component/gdrive-adapter
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/gdrive-adapter
default_branch_control_is_active: no

active_prompt: crates/haze-gdrive-adapter/control/prompt.md
active_report: crates/haze-gdrive-adapter/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P3 OAuth Expiry

wave: W1
phase: FIX-GDA-GDA-P3-REVIEW
implementation_status: COMPLETE_AFTER_CI_FIX
fix_status: REVIEW_FIX_NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
ci_status: CI_GREEN_ON_REVIEWED_SHA
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

review_candidate:
- code_bearing_sha: cf86aba890df6dcfb95f1d285677ceacf96a7772
- clean_review_report_blob: ec281ebc5f3f5343887d339d7f676977587ab738
- ci_run_id: 29456659157
- ci_run_number: 2026

required_fix:
- deterministic explicit token time boundary
- post-refresh minimum-lifetime validation against caller now
- reject and do not cache unusable refreshed tokens
- focused expired/usable/too-short cached and refreshed lifecycle tests
- preserve read-only credentials, redaction and fake-only boundaries

protected_scope:
- no next GDrive product phase
- no Server/API/Storage/scheduler/Deployment/sibling/workflow changes
- no real credentials, network calls or test weakening

next_gate:
- FIX_COMPLETE plus full exact-SHA green CI -> repeat OAuth/security clean review
- unresolved expiry invariant -> FIX_NEEDS_MORE
