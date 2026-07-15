# W1-DEP-P5A-MIGRATION-OWNERSHIP-REVIEW

Before starting, name this worker chat exactly:

`deployment — W1 DEP-P5A Migration Ownership Review`

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: architect-reviewer
Phase: DEP-P5A-MIGRATION-OWNERSHIP-REVIEW

This is an architecture/operations policy gate, not DEP-P5A implementation.

Do not merge, change draft state, rewrite history, modify sibling branches, add services, change runtime configuration, run migrations, add secrets, or begin Worktree deployment fan-in.

## Synchronized baseline

- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- deployment post-sync SHA: `54e0b8b84e06e7475dc99ea25b22ddd248bb98c2`;
- pre-sync report blob: `ad4156e26fae85bfa1049e71ea9738640b92a715`;
- Component CI run `29400618638`, number `1954`, success;
- PR #52 remains open, draft and unmerged.

Accepted dependencies:
- Storage migration/runtime-state contract accepted;
- Server Worktree runtime and HTTP operator surface accepted;
- API-P8 accepted;
- CLI-P6A accepted.

## Policy source to review

Primary source:
- `deploy/docs/migrations-backup-restore.md`.

Relevant contract:
- `deploy/docs/component-contract.md`.

Current documented policy states:
- Storage owns migration contents and schema design;
- the Server binary/Compose service does not auto-run migrations;
- the operator runs SQLx migrations explicitly;
- Deployment owns sequencing around stopping writers, coordinated PostgreSQL/object-store backup, migration, startup and verification;
- writers remain stopped or quiesced during backup/migration/restore;
- automatic migration in Compose/Dockerfile is forbidden until separately accepted;
- production secrets and database URLs remain operator-local and untracked.

## Review questions

Decide whether the existing policy is explicit and sufficient to unblock DEP-P5A.

Verify:
1. Exactly one execution owner exists: the human/operator invoking the documented manual SQLx command.
2. Storage owns migration contents/schema, not execution timing.
3. Deployment owns operational sequencing and documentation, not schema or Server internals.
4. Server startup does not implicitly migrate and failure to migrate cannot be mistaken for successful rollout.
5. All current and future writers, including hosted Worktree runtime/adapters, must be stopped or quiesced before the coordinated backup/migration window.
6. PostgreSQL metadata and object-store content are treated as one recovery window.
7. Backup occurs before migration; startup occurs only after successful migration.
8. Verification distinguishes process health, readiness and actual migration/rollout success.
9. Rollback is restore-based/operator-approved; no automatic down migration, reset, drop, cleanup or destructive fallback is implied.
10. Credentials, URLs, dumps and archives remain outside tracked files and public reports.
11. Local Compose guidance is not represented as production readiness proof.
12. DEP-P5A may consume this policy without inventing automatic migration behavior or direct database ownership.
13. Any ambiguity that could permit two owners, startup-time migration, concurrent writers, mismatched backup windows or automatic destructive recovery is blocking.

## Allowed action

Prefer review-only. Do not edit files if the existing contract is sufficient.

If a narrowly scoped wording defect prevents a clear architecture verdict, do not silently rewrite policy. Report `ARCHITECT_NEEDS_DECISION` or `ARCHITECT_NEEDS_POLICY_FIX` with the exact ambiguity and proposed boundary.

## Report

Write `deploy/control/report.md` with:
- `REPORT_TYPE: ARCHITECTURE_REVIEW`;
- `phase_id: DEP-P5A-MIGRATION-OWNERSHIP-REVIEW`;
- `chat_name: deployment — W1 DEP-P5A Migration Ownership Review`;
- status `ARCHITECT_ACCEPT`, `ARCHITECT_NEEDS_DECISION`, `ARCHITECT_NEEDS_POLICY_FIX`, `ARCHITECT_BLOCKED_BY_CONTRACT`, or `ARCHITECT_BLOCKED_BY_TOOLING`.

For `ARCHITECT_ACCEPT`, explicitly pin:
- migration execution owner;
- schema owner;
- operational sequence owner;
- writer-quiescence rule;
- backup consistency rule;
- rollback rule;
- secret handling rule;
- whether DEP-P5A product/config/runbook work is now authorized.

Do not implement DEP-P5A, modify product/config/runbook files, or claim merge readiness.
