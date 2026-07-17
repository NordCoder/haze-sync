# Control State

component: gdrive-adapter
repository: NordCoder/haze-sync
branch: component/gdrive-adapter
status: PROMPT_READY
active_prompt: crates/haze-gdrive-adapter/control/prompt.md
active_report: crates/haze-gdrive-adapter/control/report.md
active_agent_role: implementation-worker
agent_execution_id: gdrive-adapter-GDA-GDA-P4-impl-20260717102652-509ee26f

wave: W1
phase: GDA-GDA-P4-LONG-RUNNING-RUNTIME
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN

accepted_inputs:
- p2_code_sha: 746dc8790643e13e85553ff94f6b124a5c686127
- p2_clean_report_blob: e78d1d5141c17cd60f0487d251cf73a8c985e6de
- p2_ci_run_id: 29539680811
- oauth_code_sha: 7f00a60641ca157907d0e75e4ab1bb47c05f03c9
- api_sha: c60c3976696da1970d539e5cff6e9f74a61fc10e
- server_sha: c023b83e1e6f502e7d2261acccb871dd5588edf1
- storage_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
- architecture_report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4

required_surface:
- standalone long-running runtime
- concrete GDrive-local provider, token and Server gateways as needed
- serialized scheduler with full-scan correctness
- mode gating and durable crash-safe progress
- bounded backoff, degraded states and graceful shutdown
- deterministic fake and loopback-only tests

protected_scope:
- no direct database access
- no sibling owner semantic changes
- no public status/control, CLI or Deployment work
- no workflow changes, live credentials or external-network CI

next_gate:
- SELF_ACCEPT with exact-SHA green CI -> focused runtime clean review
- CI_RED -> focused component fixer
- contract or tooling blocker -> stop with exact evidence
