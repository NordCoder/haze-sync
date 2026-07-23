# Implementation Plan: common

## Current state

`haze-sync-common` is already more than a scaffold. The current crate exposes:

- `VaultPath` validation and normalization;
- `AdapterId`, `RevisionId`, `OperationId`, and `ConflictId` newtypes;
- `Sha256` / `ContentHash` representation;
- `AdapterRole` and `AdapterMode` enums;
- `ValidationError` safe error vocabulary;
- `security::SecretString` redaction wrapper;
- unit tests for the existing primitives.

The previous component docs were scaffold-level and did not fully describe the existing public contract. This plan treats the existing code as the implementation baseline and focuses future work on contract hardening, test completeness, and safe shared primitives that unblock sibling components.

## Target state

The target state for `common` is a small, deterministic, runtime-free Rust crate that provides stable shared primitive contracts for all Haze Sync components.

The component is complete enough for V1 when:

- all shared primitive wire formats are documented and tested;
- no sibling component imports raw strings where a shared primitive is required;
- API/Server/Core/Storage/CLI/adapters can use common primitives without pulling runtime dependencies;
- adapter role/mode vocabulary is stable across Rust components and compatible with TypeScript client contracts;
- validation errors are safe for public mapping;
- secret-bearing values redact by default;
- no Core policy, storage behavior, route behavior, provider behavior, or CLI behavior leaks into this crate.

## Implementation phases

### CMM-P1 — Component contract and planning normalization

Status: in progress through this documentation pass.

Goal:

```text
Replace scaffold component docs with a real contract, dependency map,
implementation plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
crates/haze-sync-common/docs/**
```

Deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial architecture decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no public API changes;
- no CI/workflow changes;
- no sibling component changes.

Checks:

- GitHub diff inspection only when working through connector;
- no shell checks required for docs-only work.

### CMM-P2 — Public primitive audit and doc/test alignment

Goal:

```text
Audit current common primitives against the documented contract and close
small gaps in tests or rustdoc without changing intended behavior.
```

Allowed scope:

```text
crates/haze-sync-common/src/**
crates/haze-sync-common/docs/**
```

Likely work:

- verify all public re-exports match the contract;
- ensure rustdoc comments clearly distinguish validation, representation, and policy;
- add missing unit tests for current edge cases;
- ensure serde wire values are stable;
- ensure no public formatting leaks sensitive values;
- document any discovered contract mismatch.

Non-goals:

- no new runtime behavior;
- no sibling crate edits;
- no new dependency on Core/API/Storage/Server;
- no broad naming churn unless required for contract correctness.

Acceptance:

- existing public API remains compatible unless a contract change is explicitly reported;
- tests cover all current primitive categories;
- cargo checks for `haze-sync-common` are green when runnable.

### CMM-P3 — VaultPath contract hardening

Goal:

```text
Make path normalization and rejection behavior fully explicit, tested,
and ready for adapters that receive paths from HTTP, filesystem scans,
Google Drive, and Obsidian state.
```

Allowed scope:

```text
crates/haze-sync-common/src/path.rs
crates/haze-sync-common/src/error.rs
crates/haze-sync-common/docs/**
```

Likely work:

- expand reserved path test matrix;
- verify percent-decoding cannot bypass traversal/null-byte checks;
- verify Windows and Unix escape forms are rejected;
- decide whether reserved conflict materialization paths such as `_haze_conflicts/**` are syncable or reserved;
- document the decision in `decisions.md` if clarified;
- avoid filesystem-specific behavior beyond string/path validation.

Contract-change triggers:

- changing whether `_haze_conflicts/**` is allowed;
- changing dot/duplicate separator normalization;
- adding provider-specific path exceptions;
- exposing raw rejected input in errors.

Acceptance:

- path behavior is deterministic and documented;
- invalid path tests cover adapter-relevant inputs;
- downstream components can depend on `VaultPath` without duplicating validation.

### CMM-P4 — Identifier and hash contract hardening

Goal:

```text
Finalize shared identity and content hash representation for Core, Storage,
API, Server, CLI, Worktree, GDrive, and Obsidian client contracts.
```

Allowed scope:

```text
crates/haze-sync-common/src/ids.rs
crates/haze-sync-common/src/hash.rs
crates/haze-sync-common/src/error.rs
crates/haze-sync-common/docs/**
```

Likely work:

- verify ID max length and allowed character set are suitable for all components;
- ensure typed IDs reject missing prefixes;
- ensure canonical hash output is always `sha256:<lowercase hex>`;
- decide whether additional IDs belong in common, such as `BlobId`, `CursorId`, or `TombstoneId`;
- keep ID generation outside common unless explicitly scoped.

Contract-change triggers:

- adding new ID types used by multiple components;
- changing prefix rules;
- changing hash wire format;
- adding hash computation helpers.

Acceptance:

- all existing ID/hash contracts are tested;
- any new shared ID type has clear downstream owners and tests;
- no storage or Core behavior is introduced.

### CMM-P5 — Adapter mode, role, and security primitive hardening

Goal:

```text
Stabilize cross-component adapter role/mode vocabulary and secret wrapper
behavior before adapter runtime implementation expands.
```

Allowed scope:

```text
crates/haze-sync-common/src/adapter.rs
crates/haze-sync-common/src/security/**
crates/haze-sync-common/src/error.rs
crates/haze-sync-common/docs/**
```

Likely work:

- verify adapter modes match system rollout semantics;
- add declarative helpers only when they remain policy-free;
- ensure role/mode serde wire values are stable;
- audit whether `ReadonlyAgent` should remain a V1 role or be renamed/removed by contract change;
- verify `SecretString` does not serialize or leak through formatting;
- decide whether common needs additional redaction wrappers for token hashes or public-safe labels.

Contract-change triggers:

- changing adapter role/mode vocabulary;
- adding permission enforcement to common;
- adding token verification, hashing, loading, persistence, or generation;
- changing redaction semantics.

Acceptance:

- role/mode vocabulary is explicit and tested;
- security wrappers remain small and non-runtime;
- downstream components know which behavior they must enforce themselves.

### CMM-P6 — Shared primitive compatibility fixtures

Goal:

```text
Provide lightweight compatibility fixtures or examples that downstream Rust
components and TypeScript clients can use to verify wire-format agreement.
```

Allowed scope:

```text
crates/haze-sync-common/**
```

Possible deliverables:

- JSON fixture examples for `VaultPath`, IDs, hashes, adapter roles/modes, and validation errors;
- Rust tests that assert fixture compatibility;
- documentation for TypeScript mirror types in the Obsidian plugin.

Non-goals:

- no TypeScript edits in this component phase;
- no generated-code pipeline unless explicitly accepted;
- no full API DTO definitions;
- no adapter behavior.

Contract-change triggers:

- needing to edit `apps/haze-obsidian-plugin` or `haze-sync-api` to keep fixtures useful;
- discovering DTO ownership ambiguity between `common` and `api`.

Acceptance:

- downstream components have stable examples to mirror;
- fixtures do not duplicate Core policy or API DTOs;
- compatibility risk is reduced before Obsidian/GDrive phases.

## Dependency gates

`common` should be planned and stabilized before later component plans rely on primitive semantics.

Blocking relationships:

```text
core
  depends on paths, IDs, hashes, and safe errors

api/server
  depend on path/hash/adapter/security primitives and safe error mapping

storage
  depends on IDs/hashes/path values and serialized forms

worktree/gdrive/obsidian/cli
  depend on path/hash/adapter mode/security primitives
```

A Common contract change after downstream components have implemented against it should trigger an Architect review or an explicit compatibility phase.

## Known risks

- Path contract ambiguity around `_haze_conflicts/**` may affect conflict materialization and adapter scans.
- `AdapterMode` helpers can accidentally become policy enforcement if expanded too far.
- `AdapterRole` vocabulary may drift from Server/API authorization if not kept explicit.
- Adding shared DTOs to `common` can blur ownership with `api`.
- Adding token behavior to `common` can blur ownership with Server/API/CLI security flows.
- TypeScript mirror types in the Obsidian plugin may drift from Rust wire values without fixtures or compatibility checks.

## Deferred work

Deferred until sibling component planning clarifies demand:

- additional shared IDs such as tombstone/blob/cursor IDs;
- generated or checked TypeScript mirror fixtures;
- stricter adapter label/name validation;
- explicit public-safe display wrappers for non-secret operational labels;
- common-level fixture files for wire compatibility;
- decision on `_haze_conflicts/**` path reservation vs sync visibility.

## Completion criteria for the component

`haze-sync-common` is V1-ready when:

- component contract and dependency map are complete;
- all primitive types used by multiple components are documented;
- all primitive validation and wire formats are tested;
- no runtime/persistence/provider dependencies exist;
- no Core/API/Storage/Server policy is implemented locally;
- downstream components can use common primitives without duplicating validation logic.
