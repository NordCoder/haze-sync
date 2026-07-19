WORKFLOW: workflow-minimal
PROMPT_ID: 8-cli-orch-plan-20260719-01
CHAT_KEY: 8 cli
ROLE: implementation-worker
COMPONENT: cli
REPOSITORY: NordCoder/haze-sync
CONTROL_BRANCH: agentic/workflow-minimal-acceptance/cli
PRODUCT_BRANCH: component/cli
INPUT_COMMIT: b174f333219d150704af4d717d78c7ede8700211
CURRENT_COMPLETION_PLAN: haze-sync-v1-completion-plan.md
WORK_ITEM: CLI-R1
PLAN_SECTIONS:
- §4.6 CLI-R1
- §6 Wave R1
- §8 First dispatch batch

# Objective

Complete CLI-R1: replace deferred executable clients with bounded configuration loading and a concrete authenticated HTTP transport using the component's existing contracts.

# Verified context

Current HEAD is `b174f333219d150704af4d717d78c7ede8700211`. The normal executable path still constructs `CliConfig::default()`, `DeferredHttpClient`, and `DeferredWorktreeClient`. Configuration source types and passive Server/Worktree client contracts already exist.

# Read

- `haze-sync-v1-completion-plan.md`, CLI-R1 and first dispatch batch;
- `crates/haze-sync-cli/docs/component-contract.md`;
- `crates/haze-sync-cli/docs/implementation-plan.md`;
- `crates/haze-sync-cli/docs/implementation-log.md`;
- `crates/haze-sync-cli/docs/dependency-map.md`;
- `crates/haze-sync-cli/src/main.rs`;
- `crates/haze-sync-cli/src/config.rs`;
- `crates/haze-sync-cli/src/server_api.rs`;
- `crates/haze-sync-cli/src/worktree_api.rs`;
- `crates/haze-sync-cli/src/doctor_live.rs`;
- `crates/haze-sync-cli/src/output.rs`.

# Scope

Allowed:
- `crates/haze-sync-cli/**`;
- minimal `Cargo.lock` update only when directly caused by `crates/haze-sync-cli/Cargo.toml`.

Forbidden:
- `main`, sibling branches, another actor control branch;
- changes to Server, API, Core, Storage, adapters, deployment, or CI;
- later CLI-R2/CLI-R3 behavior;
- direct database or provider access;
- sensitive values in output or diagnostics;
- unbounded network behavior, redirect following without an explicit safe rule, history rewriting, or unrelated cleanup.

# Required work

1. Verify the product HEAD matches `INPUT_COMMIT`.
2. Implement the existing configuration precedence and profile/source contracts with bounded input handling.
3. Implement a concrete HTTP transport for the existing Server and Worktree client traits, including authenticated requests, explicit timeouts, response-size bounds, redirect handling, status mapping, and malformed-response handling.
4. Wire the normal executable path to the real configuration and transport while preserving offline and not-configured behavior.
5. Keep output secret-safe and exit codes truthful.
6. Add local fake-server tests for precedence, source selection, request authentication, timeout, oversized and malformed responses, redirects, common HTTP failures, and unavailable Server behavior.
7. Commit and push only the bounded CLI-R1 implementation. Do not create another assignment.

# Checks

- `cargo fmt --check`;
- `cargo check -p haze-sync-cli`;
- `cargo test -p haze-sync-cli`;
- `cargo clippy -p haze-sync-cli --all-targets -- -D warnings`;
- verify the executable no longer defaults to deferred clients;
- verify the final diff is inside authorized scope;
- record every check not run and why.

# Completion

Write `.agentic/report.md` on the control branch with matching identity, work item, plan source/sections, input/output commits, changed files, checks, limitations, remaining work, and blockers. Create matching `.agentic/done.json` and publish report plus done marker together in one commit. Reread both. Before product push or another external side effect, reread this prompt and verify that `PROMPT_ID` is unchanged.