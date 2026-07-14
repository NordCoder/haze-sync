# W1-CLI-P6A-FUNCTIONAL-REVIEW

Before starting, name this worker chat exactly:

`cli — W1 CLI-P6A Worktree Operator Review`

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: clean-code-reviewer
Phase: CLI-P6A-FUNCTIONAL-REVIEW

Do not merge, change draft state, rewrite history, modify sibling branches, begin Deployment work, or perform unrelated cleanup.

Candidate:
- synchronized baseline `8a3012a20440066422e7ad6c4e52d1a859b1bd51`;
- initial implementation `6c4c2ba4a66999e02542083512587d0d6ad8d437`;
- final fixed SHA `70c3567f587a249a180eb8b9abb155065d197e5c`;
- implementation report blob `5df1ebcc3696b1d61d514e005b2dbaebe4213622`;
- fixer report blob `e55126c8d84d1aa568cd004eaff4503b617504e6`;
- Component CI `29342522606`, run `1948`, success.

Formatting, rustfmt, naming taste and style are non-blocking.

Review substantive correctness only:

1. Accepted API-P8 blobs remain byte-identical; no API control files or semantic edits.
2. CLI uses accepted API DTOs directly without divergent public vocabulary.
3. Exact commands exist: `haze-sync worktree status` and `haze-sync worktree sync-once`.
4. Existing status/adapters/doctor behavior remains unchanged.
5. Usage explains one bounded server-owned DryRun request.
6. Unknown, trailing and forbidden control arguments are rejected without echo.
7. Client cannot set path, budget, mode, force, JSON, ticket, generation or request id.
8. Status request is exact GET `/v1/admin/worktree/status`.
9. Sync request is exact POST `/v1/admin/worktree/sync-once` with strict `{}`.
10. Deferred transport cannot return fake live success.
11. No token CLI flags, secret persistence, DB/filesystem/provider access, local runtime or Worktree dependency.
12. No retry, polling, wait, task, ticket or generation behavior.
13. Valid 200 Worktree status renders success for Disabled, Running+Busy and Failed lifecycle facts.
14. Running+Busy remains readiness `ready`; CLI does not derive readiness.
15. Rendering exposes only accepted public fields and safe errors.
16. Accepted/202 means queued/submitted, explicitly not completed.
17. Busy/409, lifecycle-unavailable/503 and Failed/500 are non-zero.
18. 401/403 map safely.
19. HTTP/body mismatches are rejected; HTTP alone cannot imply success.
20. No raw bodies, headers, credentials, paths, roots, fingerprints, DB URLs, provider data, tickets or generations are output.
21. Tests cover parser, request contract, DTO vocabulary, status cases, all sync outcomes, mismatches, secrecy and legacy regressions.
22. Fixer changed only `crates/haze-sync-cli/src/worktree_api.rs`; no check weakening.
23. No Server/Worktree/Storage/Core/GDrive/Deployment, migration or workflow changes.
24. Exact final SHA has green CI; later commits are control-only.

Do not modify code unless a concrete functional, security, contract or scope defect remains.

Write `crates/haze-sync-cli/control/report.md` with:
- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: CLI-P6A-FUNCTIONAL-REVIEW`;
- `chat_name: cli — W1 CLI-P6A Worktree Operator Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker remains, use `CLEAN_ACCEPT` and authorize Orchestrator to resolve the next development phase. Do not begin Deployment work or claim merge readiness.
