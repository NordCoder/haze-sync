# Common V1 Compatibility Fixtures

## Purpose

`fixtures/common-primitives-v1.json` is the language-neutral compatibility reference for primitive wire values owned by `haze-sync-common`.

It exists so downstream Rust components and TypeScript clients can test that they agree on:

- normalized `VaultPath` output;
- shared identifier strings;
- canonical SHA-256 output;
- complete adapter role and mode vocabularies;
- declarative adapter mode capability flags;
- safe validation error codes and messages.

The fixture is verified by `tests/compatibility_fixtures.rs` against the actual Rust implementations.

## Scope

The fixture contains Common-owned primitives only. It does not define:

- HTTP request or response envelopes;
- API DTO ownership;
- status codes or route behavior;
- Core revision, conflict, delete, or ordering policy;
- adapter authorization or runtime enforcement;
- storage records;
- provider payloads;
- token or credential behavior.

A downstream component may embed these primitive strings inside its own DTOs, but the DTO remains owned by that component.

## Fixture structure

`schema_version` identifies the fixture schema. Version `1` contains:

- `vault_paths`: accepted input and canonical serialized output pairs;
- `identifiers`: one safe example for every currently shared ID type;
- `content_hashes`: accepted input and canonical lowercase `sha256:<hex>` output pairs;
- `adapter_roles`: every accepted `AdapterRole` wire value;
- `adapter_modes`: every accepted `AdapterMode` wire value plus declarative Core read/write flags;
- `validation_errors`: every safe serialized `ValidationError` code and its public message.

The examples are deterministic fixtures. They are not production identifiers, credentials, server URLs, user vault data, or environment-specific paths.

## Rust consumers

Rust components should use the public `haze-sync-common` types directly. They may load the JSON fixture in compatibility or integration tests when they need a language-neutral reference.

The fixture must not replace normal constructor validation. For example, consumers should still parse paths through `VaultPath`, IDs through their typed constructors, and hashes through `Sha256` / `ContentHash`.

## TypeScript clients

TypeScript clients may mirror the fixture in tests or use it to verify local literal unions and validators.

Recommended rules:

- treat `adapter_roles` and each `adapter_modes[].wire` value as exact lowercase string literals;
- serialize hashes only in canonical lowercase `sha256:<hex>` form;
- expect normalized vault-relative path output, not local absolute paths;
- treat validation error `wire` values as primitive codes, not as a complete API error envelope;
- use mode capability flags only as shared declarative facts; authorization and runtime permission checks remain owned by Server/API/adapters;
- do not infer additional accepted values from examples that are not present in the fixture or component contract.

The Obsidian plugin should consume API-compatible DTOs from its own component/API contract while mirroring these primitive values where those DTOs contain Common-owned fields.

## Versioning

Breaking changes to primitive wire values require an explicit Common contract change and a new versioned fixture file rather than silently redefining V1.

Adding another non-breaking example may update the V1 fixture only when it does not change the accepted vocabulary or canonical representation. The Rust compatibility tests must change in the same code-bearing commit.

## Security

Compatibility fixtures must remain safe to publish and log. They must not contain:

- real credentials or token-like values;
- bearer/OAuth material;
- database URLs;
- HTTP endpoints;
- local absolute filesystem paths;
- production provider payloads;
- real user vault content.
