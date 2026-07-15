# W1-DEP-P5A-FUNCTIONAL-SECURITY-REVIEW

Before starting, name this worker chat exactly:

`deployment — W1 DEP-P5A Functional Security Review`

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: clean-code-reviewer
Phase: DEP-P5A-FUNCTIONAL-SECURITY-REVIEW

Do not merge, change draft state, rewrite history, modify sibling branches, or perform unrelated cleanup.

## Candidate

- synchronized baseline: `54e0b8b84e06e7475dc99ea25b22ddd248bb98c2`;
- initial DEP-P5A SHA: `141878e549d9b05ffe0f4c1019bb31ee14b1bacd`;
- final documentation-aligned SHA: `d14b5f04177eae13d03b386b6c73da66921835e3`;
- implementation report blob: `829fa86932d3b63ac8bfd578fd3f295f4218b135`;
- fixer report blob: `b7f737a9681678688f6f75dfaa0a2cd2b6319922`;
- architecture report blob: `737a3395945d94479c19b27d771384b57c267d06`;
- authoritative Component CI run `29406615806`, number `1961`, success.

Formatting and wording taste are non-blocking. Review functional, operational, security, secrecy and scope correctness only.

## Review checklist

1. Base `deploy/docker-compose.yml` remains Worktree-disabled and has no Worktree bind mount.
2. Exactly one opt-in override exists for Worktree hosting.
3. Override extends only the existing `server` service.
4. `HAZE_SYNC_WORKTREE_HOST_PATH` is required when the override is used and has no silent tracked default.
5. Container target is exactly `/var/lib/haze-sync/worktree`.
6. Accepted `HAZE_SYNC_WORKTREE_PATH` and `HAZE_SYNC_WORKTREE_ADAPTER_MODE` keys are preserved without aliases.
7. Adapter mode defaults to `disabled`.
8. Bind mount defaults read-only.
9. Writable bind and write-capable adapter mode are separate explicit operator gates.
10. No automatic `export_only`, `bidirectional` or other write-capable rollout exists.
11. No migration runner, startup migration, entrypoint mutation, healthcheck side effect, background service or hidden database action was introduced.
12. PostgreSQL, object-store and public bind topology remain unchanged.
13. `.env.example` contains placeholders only and no real path, URL, credential, token or secret.
14. Runbook invocation, target path, UID 10001 and permission guidance are exact and non-world-writable.
15. Process health, readiness, Worktree status and rollout approval remain distinct gates.
16. Documentation does not claim CLI-P6A has proven live HTTP transport if it remains deferred.
17. Human/operator remains the sole migration execution owner.
18. Storage remains schema/migration-content owner; Deployment owns sequencing only.
19. All writers are stopped or explicitly quiesced for backup, migration and restore windows.
20. Complete recovery set coordinates PostgreSQL, object store and Worktree whenever Worktree may contain authoritative or unreplicated content.
21. Rollback remains operator-approved coordinated restore only; no down/reset/drop/destructive fallback.
22. Base Compose remains clearly local-only and not production-readiness proof.
23. Host-directory and migration runbooks no longer contradict the accepted override.
24. No Server, Storage, Worktree, API, CLI, GDrive, Obsidian, migration or workflow files changed.
25. No real host mutation, remote automation, cleanup or secret material was added.
26. Exact final SHA has green Component CI; later commits are control-only.

Do not modify files unless a concrete functional, security, secrecy, contract or scope defect exists.

## Report

Write `deploy/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: DEP-P5A-FUNCTIONAL-SECURITY-REVIEW`;
- `chat_name: deployment — W1 DEP-P5A Functional Security Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker remains, use `CLEAN_ACCEPT` and authorize Orchestrator to resolve the next Deployment phase. Do not claim merge readiness.
