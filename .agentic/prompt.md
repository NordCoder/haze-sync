WORKFLOW: workflow-minimal
PROMPT_ID: 6-storage-orch-plan-20260719-01
CHAT_KEY: 6 storage
ROLE: implementation-worker
COMPONENT: storage
REPOSITORY: NordCoder/haze-sync
CONTROL_BRANCH: agentic/workflow-minimal-acceptance/storage
PRODUCT_BRANCH: component/storage
INPUT_COMMIT: bfcca9664157dffef8f3162362cbdc1770368594
CURRENT_COMPLETION_PLAN: haze-sync-v1-completion-plan.md
WORK_ITEM: STOR-R1
PLAN_SECTIONS:
- §4.3 storage — STOR-R1 — V1 migration-chain proof
- §6 Wave R1 — Contract closure and independent foundations
- §8 First dispatch batch

# Objective

Prove the complete V1 PostgreSQL migration chain for clean installation and supported sequential upgrades. Fix only migration or migration-test defects proven by the evidence. Produce exact DB-capable terminal evidence without adding Server or Core behavior.

# Context

Current `component/storage` HEAD is `bfcca9664157dffef8f3162362cbdc1770368594`. The branch contains migrations `0001` through `0011`, schema/test-support validation, Worktree migration guard coverage, and GDrive durable-state migration guard coverage. Current tests demonstrate important isolated guards, but the completion plan still requires one coherent clean-install and upgrade matrix on real PostgreSQL, including final tables, constraints, indexes, supported prior states, and rollback behavior. STOR-R1 remains incomplete until that evidence is terminally recorded.

# Inputs and evidence

- `haze-sync-v1-completion-plan.md`: STOR-R1, Wave R1, first dispatch batch;
- `component/storage` @ `bfcca9664157dffef8f3162362cbdc1770368594`;
- `migrations/0001_sync_adapters.sql` through `migrations/0011_gdrive_durable_state.sql`;
- `crates/haze-sync-storage/docs/component-contract.md`;
- `crates/haze-sync-storage/docs/implementation-plan.md`;
- `crates/haze-sync-storage/docs/implementation-log.md`;
- `crates/haze-sync-storage/docs/dependency-map.md`;
- `crates/haze-sync-storage/docs/worktree-durable-state.md`;
- `crates/haze-sync-storage/docs/gdrive-durable-state.md`;
- `crates/haze-sync-storage/src/test_support/postgres/implementation.rs`;
- `crates/haze-sync-storage/tests/stor_p10_migration_guard.rs`;
- `crates/haze-sync-storage/tests/stor_gda_p11_migration_guard.rs`;
- existing Storage repository and feature-gated PostgreSQL tests.

# Authorized scope

Allowed:

- `migrations/**` only for defects proven by STOR-R1 migration evidence;
- `crates/haze-sync-storage/src/test_support/**` for migration harness and schema-manifest verification;
- `crates/haze-sync-storage/tests/**` for clean-install, sequential-upgrade, rollback, and schema checks;
- `crates/haze-sync-storage/src/schema/**` and Storage docs only when needed to keep manifest/evidence accurate;
- package-local Storage files directly required by a proven migration defect.

Forbidden:

- `main`, sibling product branches, another actor control branch;
- Server routes, Core policy, API behavior, adapters, CLI, deployment, or CI workflow changes;
- new product features, repository redesign, destructive data conversion without an explicit supported migration contract, or broad schema expansion;
- accepting a skipped or unavailable database test as passing DB evidence;
- force-push, rebase, reset, history rewrite, or unrelated cleanup.

# Required actions

1. Verify `HEAD(component/storage) == bfcca9664157dffef8f3162362cbdc1770368594` before editing.
2. Build one deterministic PostgreSQL test matrix that proves a clean database can apply every migration in order through `0011`.
3. Prove supported prior-schema upgrades, at minimum the accepted pre-`0010` and pre-`0011` shapes represented by the current harness and migration guards.
4. Prove a failed guarded migration rolls back transactionally and preserves the prior schema/data without partial new structures.
5. Verify the final V1 schema manifest: expected tables, columns, primary/unique keys, foreign keys, check constraints, indexes, and Worktree/GDrive durable-state structures.
6. Verify repeated startup/schema preparation is idempotent for an already complete schema and fail-closed for partial or incompatible schemas.
7. Run the matrix against a dedicated real PostgreSQL test database using the accepted Storage test configuration. Do not report DB acceptance when the required database is absent.
8. Fix only exact migration or harness defects demonstrated by failing evidence, and add regression coverage for each fix.
9. Update Storage documentation only to record the verified supported upgrade boundaries and commands.
10. Commit and push the smallest coherent STOR-R1 change to `component/storage`. Do not create the next assignment.

# Required checks

- `cargo fmt --check`;
- `cargo check -p haze-sync-storage --features test-support`;
- `cargo test -p haze-sync-storage`;
- `cargo test -p haze-sync-storage --features test-support` with a dedicated reachable PostgreSQL test database;
- run the direct ignored migration guard/matrix tests explicitly where required;
- `cargo clippy -p haze-sync-storage --all-targets --all-features -- -D warnings`;
- verify the final schema manifest against migrations `0001` through `0011`;
- verify the final diff remains inside the authorized Storage/migration scope;
- verify final branch HEAD and record the exact output SHA;
- reread this prompt immediately before product push;
- report every check not run and why; a missing database is a blocker, not green evidence.

# Completion

Write `.agentic/report.md` on `agentic/workflow-minimal-acceptance/storage`. Repeat the prompt identity, role, component, work item, completion-plan source and sections, input/output commits, exact changed files, PostgreSQL environment/evidence, schema findings, checks, limitations, remaining work, and blockers.

When terminal, create `.agentic/done.json` with the same `prompt_id` and `chat_key`, and publish report plus done marker together in one control-branch commit. Reread and verify both files. Before product push or another external side effect, reread this prompt and verify that `PROMPT_ID` is unchanged.