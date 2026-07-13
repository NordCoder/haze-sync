# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: worktree — W1 WT-P11 Hosted Runtime Contract Extension
prompt_revision: verified by Orchestrator after concrete SRV-P7B4 Worktree owner-contract blocker

wave: W1
phase: WT-P11-HOSTED-RUNTIME-CONTRACT

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED_OWNER_BOUNDARY_ALREADY_DECIDED
ci_status: NOT_RUN
known_failed_checks: []

accepted_baseline:
- wt_p10_code_bearing_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
- wt_p10_ci_run_id: 29167289593
- wt_p10_ci_run_number: 1799
- wt_p10_clean_status: CLEAN_ACCEPT
- post_baseline_product_or_tooling_changes: none

reactivation_evidence:
- server_phase: SRV-P7B4-HOSTED-WORKTREE-RUNTIME
- server_blocker_report_commit: 03e15367e1363bc4718581ca2c8993be52c2231e
- server_blocker_report_blob: 7172be029d2480eb151be2151f36718a985d089d
- server_blocker_status: BLOCKED_BY_CONTRACT
- missing_contract_1: production path-free WorktreeWatcher implementation or production-complete Worktree-owned watcher factory/channel lifecycle
- missing_contract_2: scheduler-accounted manual-cycle entrypoint preserving lifecycle, counters, last-cycle state, cancellation and no-overlap

routing_decision:
- direct Worktree owner implementation phase
- no architect phase because accepted architecture already assigns watcher and scheduler semantics to Worktree
- Server remains on SRV-P7B4-WAIT-WT-P11 hold

archived_previous_slot:
- prompt_index: crates/haze-sync-worktree/control/log/20260713-160500Z-W1-WT-P10-ACCEPTED-HOLD-control-prompt.md
- prompt_blob: 5cdfdf64353bf9ae44e726fa15621c24bc705a9f
- previous_status: ACCEPTED_HOLD

phase_goal:
- add production path-free watcher lifecycle owned by Worktree
- add scheduler-accounted manual-cycle entrypoint
- preserve WT-P10 automatic polling, cancellation, no-overlap and DryRun semantics
- provide typed coarse safe outcomes and focused tests

allowed_scope:
- Worktree runtime/watcher modules and focused tests
- Worktree lib exports
- minimal manifest/lock and Worktree docs/control changes

protected_scope:
- Server, Storage, Core, API, CLI and Deployment product files
- migrations/schema
- sibling control files and workflows

completion_requirements:
- component/worktree advances with real code-bearing contract extension
- committed crates/haze-sync-worktree/control/report.md exists
- report phase WT-P11-HOSTED-RUNTIME-CONTRACT
- report chat name worktree — W1 WT-P11 Hosted Runtime Contract Extension
- exact final code-bearing SHA recorded
- authoritative Component CI green on exact final SHA, or honest blocker/failure with exact evidence

next_gate_after_implementation:
- SELF_ACCEPT plus green exact-SHA CI -> mandatory WT-P11 clean-code review
- red CI -> fixer from exact diagnostics artifact
- CLEAN_ACCEPT afterward -> explicit exact-SHA Worktree fan-in to component/server, integration CI and clean review
- SRV-P7B4 remains blocked until fan-in acceptance
