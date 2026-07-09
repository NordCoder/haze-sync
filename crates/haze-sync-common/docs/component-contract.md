# Component Contract: common

## Responsibility

`haze-sync-common` owns deterministic shared primitives that are safe to reuse across Haze Sync components.

The component provides small, JSON-serializable, validation-backed value types for:

- vault-relative paths;
- stable identifiers;
- SHA-256 content hashes;
- adapter roles and rollout modes;
- safe validation errors;
- secret-bearing in-memory wrappers that redact on formatting.

`common` exists to keep cross-component contracts stable without pulling runtime, persistence, provider, or policy dependencies into leaf components.

## Public interfaces

The public crate surface is the stable component interface.

Current public modules:

```text
adapter
error
hash
ids
path
security
```

Current public re-exports:

```text
AdapterMode
AdapterRole
ValidationError
ContentHash
Sha256
AdapterId
ConflictId
OperationId
RevisionId
VaultPath
```

Security wrappers live under `security` and may be used through the module path when callers need explicit secret-handling semantics.

## Owned value contracts

### VaultPath

`VaultPath` is a normalized path relative to the synchronized vault view.

Required behavior:

- accepts safe relative paths such as `Notes/a.md`;
- normalizes duplicate separators and `.` segments;
- may decode percent-encoded path separators before validation;
- rejects empty paths;
- rejects absolute Unix paths;
- rejects `~` and `~/...` user-home paths;
- rejects Windows drive prefixes;
- rejects backslash separators;
- rejects path traversal after percent decoding;
- rejects null bytes after percent decoding;
- rejects reserved runtime/state paths;
- serializes and deserializes as its normalized string form.

`VaultPath` must not preserve local absolute paths, OS-specific prefixes, or raw unsafe input.

The reserved runtime/state path set currently includes first path segments `_haze_runtime`, `_haze_tmp`, `state`, `logs`, and `trash`, plus temporary file suffixes `.tmp`, `.part`, and `.swp`. `_haze_conflicts/**` and `_haze_agent_outbox/**` are not reserved by `common`; they remain valid vault paths unless a higher-level runtime component ignores or reserves them in its own scope.

### IDs

Owned identifier types:

```text
AdapterId
RevisionId
OperationId
ConflictId
```

Required behavior:

- validate on construction and deserialization;
- serialize as strings;
- expose `as_str`, `Display`, `FromStr`, and `TryFrom<&str>` behavior;
- reject empty values, null bytes, unsupported characters, and overly long identifiers;
- use a total maximum length of 128 bytes for each current identifier value, including any required prefix;
- allow only ASCII letters, ASCII digits, `_`, `-`, and `.` in current identifier values;
- require exact lowercase prefixes for typed system IDs:
  - `RevisionId`: `rev_`
  - `OperationId`: `op_`
  - `ConflictId`: `conf_`
- reject missing, wrong, empty-suffix, or case-mismatched typed ID prefixes;
- keep `AdapterId` flexible enough for configured adapter names such as `iphone-anna` or `worktree-adapter`.

Current `common` ID ownership is limited to `AdapterId`, `RevisionId`, `OperationId`, and `ConflictId`. Additional IDs such as blob, cursor, tombstone, mapping, or audit IDs must stay in their owning component until they are proven to be stable cross-component primitives and accepted by an explicit contract change.

ID generation is not owned by this component unless explicitly scoped later. `common` validates and carries IDs; Core/Storage/Server decide where IDs are minted.

### ContentHash / Sha256

`ContentHash` is currently an alias of `Sha256`.

Required behavior:

- parse plain 64-character hex SHA-256 values;
- parse canonical `sha256:<hex>` values with the exact lowercase `sha256:` prefix;
- normalize output to lowercase prefixed `sha256:<hex>` form;
- accept uppercase or mixed-case hex digits only in the digest portion and normalize them on output;
- reject invalid length;
- reject non-hex characters;
- serialize using canonical prefixed form;
- deserialize from the same accepted parse inputs and re-emit canonical prefixed form;
- expose byte-level access for hashing/storage callers.

Hash computation over file bytes is not owned by `common` unless explicitly scoped. `common` owns hash representation and validation.

### AdapterRole

Adapter roles define coarse authorization identity for adapter/client classes.

Current roles:

```text
obsidian_plugin
gdrive_adapter
worktree_adapter
admin
readonly_agent
```

Required behavior:

- stable exact lowercase snake_case wire values;
- serde roundtrips with wire values;
- `Display`, `as_str`, `FromStr`, and `TryFrom<&str>` use the same wire values;
- unknown, case-mismatched, hyphenated, or otherwise non-exact role strings are rejected safely;
- `readonly_agent` remains a V1 role for non-mutating agent/client integrations until a future contract change explicitly renames or removes it.

Role enforcement is not owned by `common`; Server/API use these values to enforce authorization.

### AdapterMode

Adapter modes define staged rollout behavior.

Current modes:

```text
disabled
read_only
import_only
export_only
bidirectional
dry_run
```

Required behavior:

- stable exact lowercase snake_case wire values;
- serde roundtrips with wire values;
- `Display`, `as_str`, `FromStr`, and `TryFrom<&str>` use the same wire values;
- unknown, case-mismatched, hyphenated, camelCase, or otherwise non-exact mode strings are rejected safely;
- helper methods may expose purely declarative capability flags such as whether a mode may read from or write to Core;
- current declarative Core capability helpers are:
  - `allows_core_reads`: true for `read_only`, `export_only`, `bidirectional`, and `dry_run`;
  - `allows_core_writes`: true for `import_only` and `bidirectional`.

Mode enforcement is not owned by `common`; adapters, Server, API, and CLI must apply mode rules in their own runtime scopes.

### ValidationError

`ValidationError` is a safe public validation error vocabulary for common primitives.

Required behavior:

- serialize as stable machine-readable snake_case codes;
- provide stable safe messages;
- never carry raw invalid input;
- never expose secrets, tokens, provider payloads, database URLs, stack traces, or local absolute paths;
- remain suitable for API error mapping.

### SecretString

`SecretString` is an in-memory wrapper for token/secret-like values.

Required behavior:

- formatting through `Debug`, alternate `Debug`, and `Display` must redact the wrapped value;
- cloned values must preserve redaction behavior;
- access to the wrapped value must use explicitly named sensitive accessors;
- no serialization should be added unless a future contract explicitly defines safe behavior;
- no hashing, verification, token loading, token generation, or persistence behavior belongs here;
- additional wrappers for token hashes, public labels, or other redaction classes are not owned by `common` unless a future contract proves stable cross-component demand.

## Input contracts

All constructors and deserializers must validate their inputs before constructing public value objects.

Inputs must be treated as untrusted when they originate from:

- HTTP paths, headers, or JSON bodies;
- CLI arguments;
- provider payloads;
- local filesystem scans;
- Obsidian plugin state;
- configuration files;
- tests that model external data.

`common` should reject invalid primitive input early, but it must not decide higher-level sync behavior such as conflict outcome, delete safety, adapter authorization, or operation ordering.

## Output contracts

Public values should output stable, deterministic, safe forms:

- `VaultPath` outputs normalized vault-relative paths only;
- IDs output validated string identifiers;
- `Sha256` outputs canonical prefixed lowercase hash strings;
- `AdapterRole` and `AdapterMode` output stable snake_case wire values;
- `ValidationError` outputs safe codes/messages without raw input;
- secret wrappers output redaction markers when formatted.

Outputs must be suitable for logs, API DTOs, and CLI summaries unless the value type explicitly represents a secret and provides sensitive accessors.

## Error contracts

Public errors must be safe and must not expose:

- secrets;
- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values;
- raw provider payloads;
- database URLs;
- SQLx/raw storage errors;
- local absolute paths;
- stack traces;
- filesystem internals outside explicit safe display contracts.

`common` errors may classify validation failures, but higher-level components own translation into HTTP status codes, CLI exit codes, retry behavior, or operator-facing remediation.

## Persistence/runtime ownership

`common` owns no persistence and no runtime.

It must not:

- connect to PostgreSQL;
- access the filesystem;
- read environment variables;
- load config files;
- call Google Drive or any provider API;
- start background jobs;
- perform HTTP routing;
- compute sync outcomes;
- generate tombstones or conflicts;
- execute authorization decisions;
- own retry/backoff behavior.

## Security and secrecy rules

- Do not commit secrets.
- Do not expose tokens or token hashes in public outputs.
- Do not expose local absolute paths or database URLs.
- Do not serialize raw provider payloads unless explicitly allowed by contract.
- Secret-bearing wrappers must redact by default.
- Validation errors must not embed rejected raw input.
- Tests must use non-sensitive fixture values.

## Non-goals

`common` must not implement:

- Core revision/conflict/delete policy;
- idempotency service logic;
- operation log behavior;
- storage repositories or database models;
- HTTP DTOs beyond primitive shared values;
- Axum route helpers;
- adapter provider clients;
- Obsidian plugin runtime behavior;
- Worktree scanner/writer logic;
- CLI command behavior;
- deployment or CI behavior;
- token creation, hashing, verification, loading, persistence, or rotation;
- cryptographic hashing of content bytes unless explicitly scoped later.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- `common` primitives are deterministic and side-effect free.
- Public wire values are stable unless a contract change is explicitly accepted.
- All public constructors/deserializers validate input before constructing values.
- Public errors and formatted secret wrappers are safe to expose.
- The component remains runtime-free and provider-free.
- The component does not import sibling Haze Sync crates.
- The component never implements Core policy locally.

## Test obligations

Required test coverage:

- valid and invalid `VaultPath` cases;
- percent-decoding safety for paths;
- reserved runtime path rejection;
- explicit coverage that `_haze_conflicts/**` remains representable as a syncable vault path;
- ID parsing, prefix enforcement, unsafe value rejection, max length, allowed character set, and serde roundtrips;
- hash parsing, canonical formatting, case normalization, exact lowercase prefix behavior, invalid length/character rejection, byte access, and serde roundtrips;
- adapter role/mode exact wire values, non-exact rejection, parsing, declarative capability helpers, and serde roundtrips;
- validation error code/message safety;
- secret wrapper redaction for `Debug`, alternate `Debug`, `Display`, formatting contexts, and cloned values.

Tests must not require secrets, production configuration, live providers, a database, or the filesystem.

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- adding a new shared primitive that affects multiple components;
- changing stable wire values;
- changing path normalization rules;
- changing adapter role/mode vocabulary;
- adding token hashing/verification/loading behavior;
- adding runtime, persistence, provider, or HTTP dependencies;
- exposing raw invalid input in errors;
- moving Core policy or API DTO responsibility into `common`.
