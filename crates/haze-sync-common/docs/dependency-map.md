# Dependency Map: common

## Component role in dependency graph

`haze-sync-common` is the lowest-level shared Rust component.

It provides stable value primitives that other Haze Sync components can depend on without inheriting runtime, persistence, provider, HTTP, CLI, or policy dependencies.

## Upstream dependencies

Allowed upstream dependencies:

```text
Rust standard library
serde
serde_json in tests
```

Conditional or future upstream dependencies require contract review if they affect public behavior.

Disallowed upstream dependencies:

```text
haze-sync-core
haze-sync-api
haze-sync-storage
haze-sync-server
haze-sync-worktree
haze-gdrive-adapter
haze-sync-cli
apps/haze-obsidian-plugin
axum
sqlx
tokio
reqwest/google provider SDKs
filesystem watcher crates
CLI parsers
config/env loading crates
tracing/logging as required runtime behavior
```

Rationale:

```text
common must remain deterministic, side-effect free, provider-free, runtime-free,
and safe to import from every sibling component.
```

## Downstream dependents

Expected downstream Rust dependents:

```text
haze-sync-core
haze-sync-api
haze-sync-storage
haze-sync-server
haze-sync-worktree
haze-gdrive-adapter
haze-sync-cli
```

Expected cross-language mirror dependents:

```text
apps/haze-obsidian-plugin
  may mirror common wire values in TypeScript client types and tests
```

Downstream dependency expectations:

- Core uses common paths, IDs, hashes, adapter identity, and validation errors when making sync decisions.
- API uses common primitives in DTOs, headers, route helpers, and safe error mapping.
- Storage uses common primitives for repository inputs/outputs and serialized values, but database schema details remain in Storage.
- Server uses common primitives in runtime state, auth execution, error mapping, and service wiring.
- Worktree uses common path/hash/adapter primitives when scanning, importing, materializing, and reporting local files.
- GDrive adapter uses common path/hash/adapter/security primitives when normalizing provider files and communicating with Core/API.
- CLI uses common primitives for parsing, displaying, and validating safe operator commands.
- Obsidian plugin mirrors public wire values and API-facing primitive shapes, but does not import Rust code directly.

## Cross-component contracts

### Path contract

Owner:

```text
common
```

Consumers:

```text
api
server
core
storage
worktree
gdrive-adapter
obsidian-plugin mirror types
cli
```

Contract:

```text
VaultPath is normalized, vault-relative, slash-separated, traversal-free,
absolute-path-free, null-byte-free, and free of reserved runtime paths.
```

Sibling components must not duplicate incompatible path validation. Provider-specific normalization may happen before constructing `VaultPath`, but final internal paths must pass `VaultPath` validation.

### ID contract

Owner:

```text
common
```

Consumers:

```text
core
api
storage
server
adapters
cli
```

Contract:

```text
AdapterId is a flexible configured identifier.
RevisionId, OperationId, and ConflictId use stable required prefixes.
All IDs validate before construction and serialize as strings.
```

ID generation ownership remains outside common.

### Hash contract

Owner:

```text
common
```

Consumers:

```text
core
api
storage
server
worktree
gdrive-adapter
obsidian-plugin mirror types
cli
```

Contract:

```text
ContentHash/Sha256 validates SHA-256 representation and serializes as
canonical lowercase prefixed `sha256:<hex>`.
```

Hash computation ownership remains outside common unless explicitly changed.

### Adapter role/mode contract

Owner:

```text
common
```

Consumers:

```text
api
server
cli
worktree
gdrive-adapter
obsidian-plugin mirror types
core when needed for decision metadata
```

Contract:

```text
AdapterRole and AdapterMode provide stable wire vocabulary only.
Runtime enforcement belongs to Server/API/adapters/CLI.
```

### Safe validation error contract

Owner:

```text
common
```

Consumers:

```text
api
server
cli
all components constructing common primitives
```

Contract:

```text
ValidationError carries no raw input or sensitive payloads and can be mapped
to public HTTP/CLI output safely.
```

HTTP status mapping belongs to API/Server, not common.

### Secret wrapper contract

Owner:

```text
common
```

Consumers:

```text
server
api
cli
gdrive-adapter
obsidian-plugin conceptual mirror only if needed
```

Contract:

```text
SecretString redacts by default and exposes raw content only through explicitly
sensitive accessors.
```

Token hashing, loading, verification, storage, and rotation are not common responsibilities.

## Shared-surface cautions

The following changes require explicit contract review before implementation:

- changing any stable wire value;
- changing `VaultPath` normalization/rejection rules;
- changing ID prefix rules;
- changing hash canonical format;
- adding runtime dependencies;
- adding sibling crate dependencies;
- moving API DTO ownership into common;
- moving Core policy into common;
- adding token verification or hashing;
- serializing secret-bearing wrappers.

## Component-local files

Common component scope:

```text
crates/haze-sync-common/**
```

Typical owned implementation files:

```text
crates/haze-sync-common/src/lib.rs
crates/haze-sync-common/src/adapter.rs
crates/haze-sync-common/src/error.rs
crates/haze-sync-common/src/hash.rs
crates/haze-sync-common/src/ids.rs
crates/haze-sync-common/src/path.rs
crates/haze-sync-common/src/security/**
```

Typical owned documentation files:

```text
crates/haze-sync-common/docs/component-contract.md
crates/haze-sync-common/docs/implementation-plan.md
crates/haze-sync-common/docs/implementation-log.md
crates/haze-sync-common/docs/dependency-map.md
crates/haze-sync-common/docs/decisions.md
```

Control files are owned by the process protocol and should only be changed according to active prompt/control-slot rules.

## Contract-change notes

Current requested/known contract questions:

1. `_haze_conflicts/**` path policy
   - Current path logic reserves `_haze_runtime`, `_haze_tmp`, `state`, `logs`, `trash`, and temp suffixes.
   - System-level conflict materialization may require `_haze_conflicts/**` to be sync-visible rather than reserved.
   - Owner decision needed before conflict-materialization phases rely on path behavior.

2. `ReadonlyAgent` role
   - Current role exists in `AdapterRole`.
   - Component planning should confirm whether this is a V1 role or a scaffold convenience.
   - If removed or renamed, all downstream auth/API/CLI docs must be updated.

3. TypeScript mirror compatibility
   - Common is Rust-only.
   - Obsidian plugin may need matching TypeScript literals for paths, hashes, roles, modes, and errors.
   - Future compatibility fixtures may be needed without moving TypeScript into the Common component.

No immediate blocking contract change is required for the documentation/planning pass.
