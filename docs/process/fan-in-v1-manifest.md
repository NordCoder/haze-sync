# Haze Sync v1 Fan-In Manifest

Status: `FROZEN_INPUTS`

Created: 2026-07-22

## Integration identity

| Field | Value |
| --- | --- |
| Repository | `NordCoder/haze-sync` |
| Base branch | `main` |
| Frozen base SHA | `c1e69a664388b0cba028170e8398b9088218957d` |
| Integration branch | `integration/v1-fan-in` |
| Fan-in method | Path-scoped import from immutable source commits |
| Direct component branch merges | Forbidden |

This manifest freezes the inputs for the first controlled fan-in candidate. Later commits pushed to component branches are not included automatically. Changing a selected source requires an explicit manifest update before that component is imported.

## Frozen component inputs

| Component | Component branch | Branch head at freeze | Selected product source | Import root | Selection note |
| --- | --- | --- | --- | --- | --- |
| `common` | `component/common` | `5d38120f36ca89a1f4c34325e40d798c52dd57d6` | `a3941f35bac9544bf55be608010da6f3d1e6ac94` | `crates/haze-sync-common/` | Accepted product snapshot; later head is control-plane maintenance. |
| `core` | `component/core` | `c0157a27149644d678de0018aa28bd4f10391d2c` | `aef27fbce707de9c4da39235a959fe5a66f13139` | `crates/haze-sync-core/` | Accepted product and review snapshot; later head is control-plane maintenance. |
| `api` | `component/api` | `707ce6d2ed71e3cb1342e37420255f0b3ad1452f` | `dba43751521c32aca53729c1c8dbddf2e7d8fbfb` | `crates/haze-sync-api/` | Code-bearing snapshot with authenticated private GDrive cursor contract. Later documentation-only changes may be reconciled separately. |
| `storage` | `component/storage` | `bfcca9664157dffef8f3162362cbdc1770368594` | `3617bd1cf947fdd394f1ab29d4b992f7b8859a84` | `crates/haze-sync-storage/` | Accepted DB-capable product snapshot, including Worktree and GDrive durable state. |
| `worktree` | `component/worktree` | `897d7fd0207bb77e61c6c0ea9cec693fd4cefb13` | `526714cdfe185713a09af68fd5bddcb967a7902e` | `crates/haze-sync-worktree/` | Accepted runtime snapshot; later head clears component reports. |
| `server` | `component/server` | `e873b0aa59afbf3e301bf3ccaaa5d87e5bab1328` | `c023b83e1e6f502e7d2261acccb871dd5588edf1` | `crates/haze-sync-server/` | Accepted DB-capable runtime snapshot. Private GDrive cursor reconciliation remains an explicit integration change. |
| `cli` | `component/cli` | `cc471be07772a6e576615c626c7f0b20b03b04df` | `cc471be07772a6e576615c626c7f0b20b03b04df` | `crates/haze-sync-cli/` | Current head selected because post-acceptance commits add the real configuration loader and authenticated HTTP transport. |
| `gdrive-adapter` | `component/gdrive-adapter` | `7667c6bc01569bc089eabdc4de2ab81069672998` | `7667c6bc01569bc089eabdc4de2ab81069672998` | `crates/haze-gdrive-adapter/` | Current implementation selected. Long-running runtime composition remains incomplete and disabled. |
| `obsidian-plugin` | `component/obsidian-plugin` | `32c6f401c344f509162530ac9719eab4032f5262` | `457f1e4904456f5ddf766791e5250e36a0fd23e6` | `apps/haze-obsidian-plugin/` | Accepted product snapshot; generated build output is excluded. |
| `deployment` | `component/deployment` | `06f7a5403f624f178e7701ccf295b83090bc04ef` | `d14b5f04177eae13d03b386b6c73da66921835e3` | `deploy/` | Accepted local deployment scaffold. GDrive service is intentionally absent until its runtime is complete. |
| `github-ci` | `component/github-ci` | `da829e5da9342c32c01df015112cc7032f2e25ef` | `REBUILD_ON_INTEGRATION_BRANCH` | `.github/` | The branch is evidence only. Canonical integration CI will be reconstructed rather than imported wholesale. |

## Global import rules

The fan-in imports only component-owned product files from the selected product source.

Always exclude:

```text
**/control/**
.agentic/**
.github/workflows/component-ci.yml
**/node_modules/**
**/dist/**
**/.test-dist/**
target/**
```

Component branches must not supply the integration branch's root `Cargo.lock`. The final lockfile is generated only after dependency reconciliation across the complete workspace.

Shared root files, workspace dependency versions, global workflows and cross-component documentation are reconciled explicitly on the integration branch rather than taken from the last imported component.

## Component-owned special paths

Storage import includes its migrations required by the selected snapshot, including:

```text
migrations/0010_worktree_durable_state.sql
migrations/0011_gdrive_durable_state.sql
```

Obsidian import includes source, tests and package/build configuration, but excludes generated artifacts and installed dependencies.

Deployment import keeps Worktree and GDrive disabled by default. A permanent GDrive Compose service must not be introduced until the adapter has a real long-running runtime.

## Known integration corrections

These are not reasons to change the frozen source inputs. They must be implemented as separate, traceable integration commits after the relevant components have been imported.

1. Align Server GDrive private durable-state mapping with API `GDrivePrivateCursorStateDto` while keeping raw cursor data out of admin responses, logs, errors, `Debug` and `Display`.
2. Reconcile workspace dependency versions and feature flags before generating the final `Cargo.lock`.
3. Build one canonical integration workflow instead of importing component-specific workflow variants.
4. Keep incomplete GDrive long-running runtime and deployment service disabled while retaining buildable library and client functionality.

## Fan-in order

```text
common
core
storage
api
worktree
server
server/api private-cursor reconciliation
cli
gdrive-adapter
obsidian-plugin
deployment
github-ci reconstruction
cross-component hardening
```

## Stage 0 completion gate

- [x] Actual `main` head recorded.
- [x] Integration branch created from the recorded `main` SHA.
- [x] All eleven component branch heads recorded.
- [x] Selected product sources recorded separately from branch heads.
- [x] Import roots and global exclusions recorded.
- [x] Known cross-component blockers recorded.
- [ ] Product paths imported. This begins in the next fan-in stage.
