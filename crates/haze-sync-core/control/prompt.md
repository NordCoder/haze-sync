# W1-FIX-CORE-FMT

Component: core
Path: crates/haze-sync-core
Branch: component/core
Role: fixer-worker

Use only the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

Context: PR #43 Component CI failed at `cargo fmt --check`. `cargo check`, `cargo test`, and `cargo clippy` were skipped.

Task: fix only formatting issues in core-owned Rust files. Make the smallest rustfmt-equivalent edits. Do not change behavior, public API, docs semantics, workflow files, or sibling components.

Read:
- crates/haze-sync-core/control/state.md
- crates/haze-sync-core/control/report.md
- PR #43 changed files / compare main...component/core
- failed Component CI job steps/logs if available

Allowed files:
- crates/haze-sync-core/src/**/*.rs
- crates/haze-sync-core/control/report.md

If exact formatting cannot be determined safely through connector-only work, write BLOCKED_BY_TOOLING in the report.

Replace control/report.md with a FIXER report.
Expected status: SELF_ACCEPT_PENDING_CI or BLOCKED_BY_TOOLING.
