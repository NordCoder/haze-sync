# Branch Policy

## Rules

- Never write directly to `main`.
- Process migrations use `process/*` branches.
- Component implementation uses `component/*` branches.
- One component branch may contain multiple implementation phases, clean-code-review passes, and fixer passes.
- Workers must not modify sibling component branches.
- Force-push/history rewrite is forbidden unless explicitly authorized.

## Recommended component branches

~~~text
component/common
component/core
component/api
component/storage
component/server
component/cli
component/worktree
component/gdrive-adapter
component/obsidian-plugin
component/deployment
component/github-ci
component/docs-process
~~~

## PR policy

Until the new protocol is proven, prefer small PRs from component branches at safe checkpoints.
