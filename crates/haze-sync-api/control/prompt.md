# W1-API-P8-PRE-SYNC — Synchronize API branch with current main

Before starting, name this worker chat exactly:

`api — W1 API-P8 Main Sync`

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker
Phase: API-P8-PRE-SYNC

Do not merge PR #44, change draft state, rewrite history, modify sibling branches, or begin API-P8 product work before synchronization is complete.

## Coordinates

- current API branch head before sync: `c07c3d76990c12cac468bfda8c5dcb3637d97485`;
- current main SHA: `c1e69a664388b0cba028170e8398b9088218957d`;
- branch relation: diverged; API branch is behind main by 152 commits and has 12 API-local commits;
- accepted API-P7C code-bearing SHA: `3109c0fd9b456ca5fd8db099cd83843dae44cef9`;
- accepted Server SRV-P7B5 SHA: `1d1fc8ca62c97db041cca09dd8316370285dfba1`;
- Server clean report commit: `23b49dff38c8b6997193b6224681feada3e09c1e`;
- Server clean report blob: `2416d7280883761bf90117a7e3dbfc41147756b9`.

## Task

Synchronize `component/api` with exact main SHA `c1e69a664388b0cba028170e8398b9088218957d` using a normal merge commit. Do not rebase or rewrite history.

Requirements:

1. Preserve all accepted API-local product and control history.
2. Resolve conflicts minimally and behavior-preservingly.
3. Do not implement API-P8 routes/DTOs/manual behavior in this slot.
4. Do not modify Server, Worktree, Storage, Core, CLI or Deployment product semantics beyond what arrives from main.
5. Do not manually cherry-pick partial Server snapshots; this slot is only branch synchronization with current main.
6. Run authoritative Component CI on the exact post-sync API branch SHA.
7. Record the merge base, main SHA, resulting merge SHA, conflict paths/resolutions and exact CI evidence.

Formatting/style alone is non-blocking when exact-SHA CI is green.

Write `crates/haze-sync-api/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: API-P8-PRE-SYNC`;
- `chat_name: api — W1 API-P8 Main Sync`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Do not claim API-P8 implemented or CLEAN_ACCEPT. After a successful sync and CI, Orchestrator will open the actual API-P8 implementation slot pinned to the accepted Server contract.
