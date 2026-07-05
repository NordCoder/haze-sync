# Dependency Map: cli

## Component role in dependency graph

`haze-sync-cli` is the operator command surface.

Conceptual position:

```text
operator shell
  -> CLI parser/output/HTTP client/doctor renderer
  -> Server public API and Core doctor summaries
  -> Core/API/Storage/adapter behavior through accepted boundaries
```

CLI is not the source of truth and not a runtime owner. It should call or render existing contracts rather than bypass them.

## Upstream dependencies

### Current direct dependencies

- `haze-sync-core`
  - passive doctor models and offline doctor check constructors used by `doctor [--offline]`.

### Future project dependencies

Expected future dependencies, only when scoped:

- `haze-sync-api`
  - DTOs, public error vocabulary, and header semantics for HTTP client commands.
- `haze-sync-common`
  - adapter IDs, modes, paths, hashes, and validation helpers for command inputs.
- `haze-sync-server`
  - public HTTP runtime endpoints, not private internals.

### External dependency categories

Potential future dependencies, subject to phase review:

- CLI parser library such as `clap`, if manual parsing becomes too costly;
- HTTP client library for server-backed commands;
- serialization for JSON output;
- terminal formatting only if it does not hide safety status;
- config parsing libraries;
- secret-source helpers if accepted.

Current implementation intentionally has no external CLI parser/network dependencies.

## Disallowed direct dependencies

CLI must not directly depend on:

```text
haze-sync-storage repositories/SQLx access by default
haze-gdrive-adapter provider internals
haze-sync-worktree scanner/materializer internals
apps/haze-obsidian-plugin internals
Google Drive provider SDKs
Obsidian plugin APIs
Server private route modules as command implementation
```

Direct database/provider/local-repair access requires explicit future contract and must not be added opportunistically.

## Downstream dependents

Expected dependents:

- human operators;
- deployment/runbooks;
- E2E scripts;
- CI smoke checks;
- future local admin workflows.

Downstream scripts may eventually depend on stable JSON output and exit codes. Those should be treated as compatibility surfaces once introduced.

## Cross-component contracts

### Core ↔ CLI

- Core owns doctor models and sync semantics.
- CLI may render Core doctor reports and request Core-derived summaries through Server/API.
- CLI must not implement conflict/delete/revision/idempotency policy.

### API ↔ CLI

- API owns public DTO/header/error vocabulary.
- CLI may consume API DTOs and map safe public errors into CLI output.
- CLI must not invent public response shapes for server-backed commands.

### Server ↔ CLI

- Server owns runtime HTTP behavior.
- CLI is a client of Server public API for live status/doctor/admin/bootstrap commands.
- CLI must not reach into Server private modules for production behavior.

### Storage ↔ CLI

- Storage owns persistence.
- CLI should not access Storage directly by default.
- Any future local DB diagnostic command requires explicit local-admin contract and strict redaction rules.

### GDrive/Worktree/Obsidian ↔ CLI

- Adapter/plugin components own runtime behavior.
- CLI may ask Server/API for status/control surfaces if those contracts exist.
- CLI must not import adapter/provider/plugin internals for normal commands.

### Deployment ↔ CLI

- Deployment owns installed service files, secrets placement, and runbooks.
- CLI may support operator workflows documented by deployment, but must not silently mutate deployment state unless a deployment/admin contract accepts it.

## Integration/fan-in ownership

The following work belongs outside CLI-only leaf phases unless explicitly scoped:

- Server route additions;
- API DTO/header/error changes;
- Core doctor/policy changes;
- Storage repository/schema changes;
- GDrive/Worktree runtime behavior;
- deployment service installation;
- real provider diagnostics;
- destructive repair operations.

CLI phases may add command parsing/rendering and HTTP clients within `crates/haze-sync-cli` only when upstream contracts exist.

## Dependency rules

- Prefer Server/API calls for live behavior.
- Keep CLI read-only by default.
- Do not bypass Core policy through direct DB or provider operations.
- Do not print secrets or raw internals.
- Treat offline doctor as offline only.
- Require explicit confirmation for future admin/destructive commands.
- Any new dependency must be justified in the implementation report and reflected here.

## Contract-change notes

Current known contract questions:

1. CLI parser dependency
   - Current parser is manual and dependency-free.
   - A parser library may be useful before many commands are added.
   - This is not blocking now.

2. Secret source policy
   - Future live commands need server token handling.
   - The accepted source mechanism and redaction expectations need design before implementation.

3. Live doctor source
   - CLI should prefer Server/API diagnostic endpoints.
   - Direct DB/provider checks would require explicit local-admin contract.

4. Admin/destructive commands
   - Pause/resume, mode changes, token rotation, repair, delete unlocks, and cleanup require API/Core/Server/Storage contracts before CLI implementation.

5. Stable JSON output
   - Once added, JSON output becomes scripting API and must be tested/versioned.

No immediate blocking contract change is required for the current documentation/planning pass.
