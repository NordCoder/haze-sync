# Control State

component: cli
branch: component/cli
status: PROMPT_READY
active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: cli — W1 CLI-P6A Test Restoration Fix

wave: W1
phase: FIX-CLI-P6A-TEST-REGRESSION
implementation_status: COMPLETE_AFTER_FIX
fix_status: NOT_STARTED_TEST_RESTORATION
clean_review_status: CLEAN_NEEDS_FIX
ci_status: CI_GREEN_BUT_REVIEW_DEFECT
architect_status: ARCHITECT_ACCEPT
known_failed_checks: []

candidate:
- baseline: 8a3012a20440066422e7ad6c4e52d1a859b1bd51
- current_sha: 70c3567f587a249a180eb8b9abb155065d197e5c
- review_report_blob: 93327542ab208bd57521b6372e29ac14f253a616
- ci_run_id: 29342522606
- ci_run_number: 1948

required_fix:
- restore legacy commands.rs and main.rs tests from baseline
- narrowly adapt for new Worktree variant
- add explicit Disabled and Failed HTTP 200 success rendering tests
- preserve Running+Busy and all current Worktree tests
- keep API-P8 blobs exact

next_gate:
- FIX_COMPLETE plus green exact-SHA CI -> final CLI-P6A review
- product defect exposed by restored test -> focused correction with evidence
- Deployment remains blocked
