# Control State

component: deployment
branch: component/deployment
status: PROMPT_READY
active_prompt: deploy/control/prompt.md
active_report: deploy/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: deployment — W1 DEP-P5A Documentation Alignment Fix

wave: W1
phase: FIX-DEP-P5A-DOCUMENTATION-ALIGNMENT
implementation_status: SELF_NEEDS_FIX
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_GREEN_CANDIDATE
architect_status: ARCHITECT_ACCEPT
known_failed_checks: []

candidate:
- code_bearing_sha: 141878e549d9b05ffe0f4c1019bb31ee14b1bacd
- implementation_report_blob: 829fa86932d3b63ac8bfd578fd3f295f4218b135
- ci_run_id: 29404284889
- ci_run_number: 1959
- ci_conclusion: success

remaining_fix:
- minimally reconcile deploy/docs/host-directory-layout.md
- minimally reconcile deploy/docs/migrations-backup-restore.md
- preserve existing Compose override, env defaults and architecture
- no product or workflow changes

next_gate:
- FIX_COMPLETE plus green exact-SHA CI -> focused DEP-P5A functional/security review
- unresolved documentation contradiction -> focused follow-up only
