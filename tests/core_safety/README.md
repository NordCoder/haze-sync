# W3 Core Safety Test Strategy

This directory documents the Wave 3 Core safety coverage added by `W3-P8 — Core safety tests`.

## What runs by default

Default test coverage is limited to behavior that already exists on current `main` at the W3-P8 base SHA. These tests are fake-backed or DTO-only and do not require PostgreSQL, real providers, network access, runtime services, secrets, or W3-F1 fan-in wiring.

The default coverage currently asserts:

- stale-base and unknown/null-base writes with different existing content do not silently overwrite the current revision;
- rejected stale/unknown writes do not store new content blobs or append operations in the fake Core boundary;
- delete idempotency fingerprints can replay the same DELETE request and reject a different DELETE request with the same key;
- public API errors for conflict, unsafe delete, and idempotency conflict serialize as safe public JSON;
- storage row models represent tombstones and conflicts as metadata records rather than embedded file content.

## Ignored/spec-only coverage

Some W3 safety behavior belongs to sibling branches that may not be merged when this phase runs. Those cases are represented as ignored spec placeholders. They compile, but they do not run by default and intentionally do not implement production behavior.

Ignored/spec-only targets include:

- stale base `conflict_saved`;
- unknown base `conflict_saved`;
- null base with existing different content `conflict_saved`;
- `accept_current`;
- `accept_conflict`;
- `keep_both`;
- `mark_resolved`;
- conflict materialization under `_haze_conflicts/open`;
- rejection of recursive `_haze_conflicts/**` source paths;
- delete tombstone creation;
- mass delete blocked by count;
- mass delete blocked by ratio;
- manual unlock behavior;
- delete never hard-deletes content.

## W3-F1 conversion notes

W3-F1 should convert each ignored placeholder into integration coverage after the relevant sibling implementations are merged. The recommended conversion path is:

1. replace placeholder `panic!` bodies with calls into the merged Core/API/storage interfaces;
2. keep default tests fake-backed where possible;
3. use ignored DB-backed tests only for behavior that truly requires PostgreSQL repositories;
4. verify conflict materialized paths always stay below `_haze_conflicts/open`;
5. verify recursive conflict inputs do not create nested conflict explosions;
6. verify deletes create tombstone and retention metadata without hard-deleting content;
7. verify public errors never expose filesystem paths, SQL/database details, provider payloads, tokens, request bodies, stack traces, or runtime internals.

This phase does not rewrite architecture docs, modify migrations, require CI changes, or add production conflict/delete behavior.
