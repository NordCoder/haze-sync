# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B5 Status Readiness
prompt_revision: verified by Orchestrator after SRV-P7B4 CLEAN_ACCEPT and green DB-capable exact-SHA CI; formatting/style excluded from blocking scope

wave: W1
phase: SRV-P7B5-STATUS-READINESS

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: NOT_RUN
known_failed_checks: []

accepted_srv_p7b4:
- code_bearing_sha: 55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe
- clean_report_commit: 5304a2733d7519a2699352686e2599ebde7d0474
- clean_report_blob: 2658e8e0bd85462ce5614c7395f1f1a7274b7258
- clean_status: CLEAN_ACCEPT
- ci_run_id: 29281569666
- ci_run_number: 1893
- ci_conclusion: success
- db_capable: yes

phase_goal:
- implement Server-owned internal status/readiness snapshot over accepted hosted Worktree runtime
- define deterministic readiness and manual availability categories
- expose passive bounded side-effect-free in-process read boundary
- preserve secret-safe coarse fields only
- prepare stable contract for later API-P8 without public routes or DTOs

readiness_policy:
- Disabled is ready/inert
- Starting is not ready
- Running is ready unless current terminal failure exists
- Busy does not make service unready
- Cancelling, Shutdown and Failed are not ready for new work
- counters alone do not fail readiness

protected_scope:
- accepted Worktree and Storage source
- migrations/schema
- Core/API/CLI/Deployment product files
- public routes, DTOs, OpenAPI, CLI output and deployment probes
- new tasks/runtimes/pollers/retries
- sibling control files and workflows

completion_requirements:
- real code-bearing Server implementation commit
- exact final SHA recorded
- authoritative DB-capable exact-SHA Component CI green or honest blocker
- committed IMPLEMENTATION report exists

next_gate_after_implementation:
- SELF_ACCEPT plus green exact-SHA CI -> focused functional status/readiness review
- substantive defect or red CI -> focused fixer
- formatting/style alone is non-blocking
- API-P8 remains blocked until SRV-P7B5 CLEAN_ACCEPT
