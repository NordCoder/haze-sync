WORKFLOW: workflow-minimal
PROMPT_ID: 2-gdrive-adapter-orch-plan-20260719-01
CHAT_KEY: 2 gdrive-adapter
ROLE: implementation-worker
COMPONENT: gdrive-adapter
REPOSITORY: NordCoder/haze-sync
CONTROL_BRANCH: agentic/workflow-minimal-acceptance/gdrive-adapter
PRODUCT_BRANCH: component/gdrive-adapter
INPUT_COMMIT: 7f85f1cc931b668e766c963da064cd857f88418c
CURRENT_COMPLETION_PLAN: haze-sync-v1-completion-plan.md
WORK_ITEM: GDA-R2
PLAN_SECTIONS:
- §4.8 gdrive-adapter — GDA-R2 — Concrete Google provider and OAuth client
- §6 Wave R1 — Contract closure and independent foundations
- §8 First dispatch batch

# Objective

Implement the bounded concrete Google authorization and Drive v3 Files/Changes client required by GDA-R2 behind the component's existing interfaces. Produce a tested implementation SHA without joining durable-state orchestration or the long-running runtime.

# Context

Current GitHub evidence shows an accepted deterministic authorization lifecycle at code-bearing SHA `7f00a60641ca157907d0e75e4ab1bb47c05f03c9`, with clean review and green Component CI run `29486777334`. The current branch HEAD is `7f85f1cc931b668e766c963da064cd857f88418c`. The existing provider boundary and fake provider are present, but no concrete Google HTTP implementation exists. GDA-R2 remains incomplete.

# Inputs and evidence

- `haze-sync-v1-completion-plan.md`: GDA-R2, Wave R1, first dispatch batch;
- `component/gdrive-adapter` @ `7f85f1cc931b668e766c963da064cd857f88418c`;
- `crates/haze-gdrive-adapter/docs/component-contract.md`;
- `crates/haze-gdrive-adapter/docs/implementation-plan.md`;
- `crates/haze-gdrive-adapter/docs/implementation-log.md`;
- `crates/haze-gdrive-adapter/docs/dependency-map.md`;
- `crates/haze-gdrive-adapter/docs/decisions.md`;
- prior accepted evidence:
  - `crates/haze-gdrive-adapter/control/log/20260716-020000Z-W1-FIX-GDA-GDA-P3-REVIEW-report.md`;
  - `crates/haze-gdrive-adapter/control/log/20260716-020000Z-W1-GDA-GDA-P3-CLEAN-REVIEW-RERUN-report.md`;
- implementation surfaces:
  - `crates/haze-gdrive-adapter/src/auth.rs`;
  - `crates/haze-gdrive-adapter/src/drive.rs`;
  - `crates/haze-gdrive-adapter/src/change_feed/provider.rs`;
  - `crates/haze-gdrive-adapter/src/error.rs`;
  - `crates/haze-gdrive-adapter/src/lib.rs`;
  - `crates/haze-gdrive-adapter/Cargo.toml`;
- official Google OAuth 2.0 web-server, security, Drive Files, Drive Changes, and error-handling documentation.

# Authorized scope

Allowed:

- bounded changes in `crates/haze-gdrive-adapter/src/auth.rs`, `src/drive.rs`, `src/change_feed/provider.rs`, `src/error.rs`, and `src/lib.rs`;
- new component-local modules under `crates/haze-gdrive-adapter/src/google_http/**` or an equivalent clearly named directory;
- package-local tests, docs, and `crates/haze-gdrive-adapter/Cargo.toml` when directly required;
- reuse the existing HTTP/TLS dependency where practical.

Forbidden:

- `main`, sibling product branches, other actor control branches, and product files outside `crates/haze-gdrive-adapter/**`;
- `Cargo.lock`, API/Server/Storage/Core/CLI/Worktree/Obsidian/deployment/CI changes;
- `src/durable_state.rs`, `src/runtime.rs`, scan/export/delete-guard orchestration, scheduler joining, or complete long-running runtime behavior;
- direct database access, Core policy, permanent hard-delete behavior, shared-drive support, shortcuts, Google Workspace document conversion, or watch channels;
- real account material or public-network access in automated tests;
- force-push, rebase, reset, history rewrite, or unrelated cleanup.

If a required dependency update needs an unauthorized shared lockfile change, stop and report the exact scope blocker.

# Required actions

1. Verify the product branch HEAD equals the declared input commit before editing.
2. Preserve the accepted authorization invariants: read-only configuration input, memory-only session material, deterministic time handling, minimum-lifetime validation, and redacted diagnostics.
3. Implement bounded authorization-code exchange and offline refresh behind a fakeable HTTP boundary.
4. Implement Drive v3 Files operations required by the existing provider interface: paginated list, metadata read, bounded download, create/upload, update, and trash.
5. Implement start-page-token retrieval and paginated Changes listing behind the existing change-feed provider boundary.
6. Enforce explicit timeout, response-size bound, redirect refusal or strict safe redirect policy, endpoint validation, deterministic encoding, and bounded JSON decoding.
7. Map 401, 403, 404, 429, and 5xx responses to stable safe categories and retry behavior without exposing raw bodies.
8. Reject or skip unsupported Google Workspace MIME types, shortcuts, shared-drive cases, and other out-of-scope shapes.
9. Add fake HTTP tests for authorization, Files, Changes, pagination, timeout, oversized response, redirect, malformed JSON, error mapping, redaction, and unsupported entries. Use no real accounts and no public network.
10. Commit and push the smallest coherent implementation to `component/gdrive-adapter`. Do not create the next assignment.

# Required checks

- `cargo fmt --check`;
- `cargo check -p haze-gdrive-adapter`;
- `cargo test -p haze-gdrive-adapter`;
- `cargo clippy -p haze-gdrive-adapter --all-targets -- -D warnings`;
- focused fake-HTTP tests for authorization, Files, Changes, limits, redirects, errors, and redaction;
- verify the final diff remains inside the authorized component scope;
- verify final branch HEAD and record the exact output SHA;
- reread this prompt immediately before product push;
- report every check not run and why.

# Completion

Write `.agentic/report.md` on `agentic/workflow-minimal-acceptance/gdrive-adapter`. Repeat the prompt identity, role, component, work item, completion-plan source and sections, input/output commits, changed files, tests, limitations, remaining work, and blockers.

When terminal, create `.agentic/done.json` with the same `prompt_id` and `chat_key`, and publish report plus done marker together in one control-branch commit. Reread and verify both files. Before product push or another external side effect, reread this prompt and verify that `PROMPT_ID` is unchanged.