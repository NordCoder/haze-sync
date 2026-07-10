# Archived active report evidence

component: obsidian-plugin
archived_at: 2026-07-10T18:30:00Z
wave: W1
phase: OBS-P9-NODE-CI
agent_role: implementation-worker
source_path: apps/haze-obsidian-plugin/control/report.md
source_blob_sha: 6f8e1afc530168c2457c03e38e0a25f9703bfd60
reported_status: BLOCKED_BY_TOOLING
workflow_code_bearing_sha: 161b412bb57546c28bf0aa7fb0d9408f5b74536d
merge_resolution_commit: 0cf1e56e759824761ce608a45b25317d963b2257
post_merge_ci_run: 29113089168
rust_job_conclusion: success
node_job_conclusion: failure
ci_artifact_id: 8235526638

Reason: the merge-conflict blocker was resolved by an authorized two-parent merge commit. The first observable Node validation run exposed a scoped TypeScript test-helper failure and requires an artifact-based fixer. The exact original report remains recoverable from source_blob_sha.
