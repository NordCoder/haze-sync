# Component Index

| Component | Path | Branch | Primary responsibility |
| --- | --- | --- | --- |
| common | `crates/haze-sync-common` | `component/common` | Shared value objects, IDs, hashes, paths, adapter/security primitives. |
| core | `crates/haze-sync-core` | `component/core` | Pure Core algorithms and service-layer primitives. |
| api | `crates/haze-sync-api` | `component/api` | HTTP DTOs, passive route-contract helpers, auth contracts, safe error shapes. |
| storage | `crates/haze-sync-storage` | `component/storage` | Schema models, SQLx repositories, object store, locks, test support. |
| server | `crates/haze-sync-server` | `component/server` | Axum runtime wiring, auth execution, transactions, route composition. |
| cli | `crates/haze-sync-cli` | `component/cli` | Operational CLI commands and local diagnostics. |
| worktree | `crates/haze-sync-worktree` | `component/worktree` | Built-in worktree adapter/runtime component. |
| gdrive-adapter | `crates/haze-gdrive-adapter` | `component/gdrive-adapter` | Google Drive adapter binary/runtime component. |
| obsidian-plugin | `apps/haze-obsidian-plugin` | `component/obsidian-plugin` | Obsidian plugin adapter component. |
| deployment | `deploy` | `component/deployment` | Local deployment and compose scaffolding. |
| github-ci | `.github` | `component/github-ci` | GitHub Actions workflows and repository automation. |
