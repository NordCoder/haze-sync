# CI Policy

CI is the objective gate after implementation and clean-code-review.

## Required honesty

Agents must distinguish:

- local command results;
- GitHub workflow results;
- commands not run because tooling cannot execute them.

Do not claim green checks unless observed.

## Default safe checks

~~~bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

Plugin and compose checks should follow the repository workflow.

## Fixer loop

If CI is red, Orchestrator writes a fixer prompt with a concise failure summary and relevant logs. Fixer changes the minimum code needed to make CI green.
