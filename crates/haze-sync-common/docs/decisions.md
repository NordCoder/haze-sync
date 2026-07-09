# Decisions: common

## 2026-07-05 — Common remains a runtime-free primitive crate

Decision:

`haze-sync-common` owns shared value representation, validation, and safe formatting only. It must remain runtime-free, provider-free, persistence-free, and policy-free.

Rationale:

All sibling components need common primitives. If `common` imports runtime crates, storage crates, provider SDKs, or Core policy, it becomes a coupling point that blocks parallel component development.

Alternatives:

- Put more helper logic in `common` for convenience.
- Allow `common` to own Core/API/storage helper abstractions.

Consequences:

- Some components must implement their own runtime enforcement using common values.
- Common APIs should stay small and deterministic.
- Cross-component policy belongs in Core/API/Server/adapters, not here.

Affected contracts:

- component contract;
- dependency map;
- all downstream component plans.

## 2026-07-05 — Common owns wire values, not enforcement

Decision:

`AdapterRole` and `AdapterMode` belong in `common` as stable wire/value vocabulary. Runtime enforcement of roles and modes belongs to API/Server/adapters/CLI.

Rationale:

Multiple components must agree on role/mode names. However, the meaning of whether a request is authorized, whether a command mutates state, or whether an adapter may propagate a change depends on runtime context and should not be hidden in shared primitives.

Alternatives:

- Move role/mode vocabulary into API.
- Move role/mode enforcement into common.
- Duplicate role/mode strings across components.

Consequences:

- Common may provide simple declarative helper methods when they remain policy-free.
- Downstream components must still enforce mode/role rules explicitly.
- Changing a role/mode wire value requires a contract change.

Affected contracts:

- API auth/header contracts;
- Server runtime auth;
- CLI adapter commands;
- Worktree/GDrive/Obsidian adapter behavior.

## 2026-07-05 — Common validates paths but does not own provider normalization

Decision:

`VaultPath` owns final vault-relative path validation and normalized internal representation. Provider-specific or filesystem-specific extraction may happen in adapters before constructing `VaultPath`.

Rationale:

Every component needs a shared definition of an internal vault path. But Google Drive folder trees, local filesystem paths, and Obsidian vault paths have different external representations. Common should validate the final internal path, not know every provider's extraction rules.

Alternatives:

- Put Google Drive path reconstruction into common.
- Put filesystem normalization into common.
- Let every component validate paths independently.

Consequences:

- Adapters may have local pre-normalization modules.
- Final internal paths must still pass `VaultPath` validation.
- Provider-specific path bugs should be fixed in the adapter unless they expose a true shared `VaultPath` contract gap.

Affected contracts:

- Worktree scanner/importer;
- GDrive path reconstruction;
- Obsidian local scanner;
- API path extraction;
- Core path-based revision decisions.

## 2026-07-09 — Conflict materialization paths remain syncable VaultPaths

Decision:

`_haze_conflicts/**` is not reserved by `haze-sync-common`. `VaultPath` accepts conflict materialization paths as ordinary vault-relative paths. Runtime components may still ignore, materialize, or treat those paths specially in their own scopes.

Rationale:

Conflict materialization is Core/adaptor policy, not a primitive validation concern. Reserving `_haze_conflicts/**` in `common` would make conflict files impossible to represent as shared vault paths and would move part of Core conflict policy into the lowest-level crate.

Alternatives:

- Reserve `_haze_conflicts/**` in `VaultPath`.
- Add a provider-specific exception for conflict paths.
- Leave the behavior implicit.

Consequences:

- Common continues to reject only runtime/state paths such as `_haze_runtime`, `_haze_tmp`, `state`, `logs`, `trash`, and temporary file suffixes.
- Core, Worktree, GDrive, Obsidian, and API layers own any recursion prevention, scan ignoring, or conflict-center policy around `_haze_conflicts/**`.
- Changing `_haze_conflicts/**` to a common-reserved path later requires an explicit contract change.

Affected contracts:

- Core conflict service;
- Worktree scanner/importer;
- GDrive path reconstruction/import;
- Obsidian local scanner/conflict center;
- API path extraction.

## 2026-07-05 — Secret wrappers redact by default and do not serialize

Decision:

`SecretString` is an in-memory wrapper that redacts through `Debug` and `Display`. It should not implement serialization, token hashing, token loading, verification, persistence, or rotation unless a future contract explicitly expands its scope.

Rationale:

The component needs a low-level redaction primitive, but secret lifecycle belongs to runtime/security components. Serialization of secret wrappers risks accidental public exposure or persistence without a scoped design.

Alternatives:

- Use plain `String` everywhere.
- Implement token hashing/verification in common.
- Serialize secret wrappers as redacted values.

Consequences:

- Callers must intentionally use sensitive accessors for verification/hashing.
- Server/API/CLI must own secret lifecycle behavior.
- Tests must verify formatting redaction.

Affected contracts:

- Server auth;
- API auth helpers;
- CLI token-related commands;
- GDrive/Obsidian token handling.

## 2026-07-05 — ID generation remains outside common

Decision:

`common` validates and carries identifiers but does not mint revision, operation, conflict, adapter, cursor, or tombstone IDs by default.

Rationale:

ID generation may require ordering, storage transactions, deterministic test fixtures, or runtime policies. Those responsibilities belong to Core/Storage/Server or dedicated component-local helpers depending on the ID type.

Alternatives:

- Add ULID/UUID generation helpers to common.
- Let every component use raw strings.

Consequences:

- ID generation ownership must be declared by downstream components.
- Common remains a validation and representation layer.
- Adding ID generators later is a contract change.

Affected contracts:

- Core revision/operation/conflict services;
- Storage repositories;
- Server route handlers;
- adapter mapping state.
