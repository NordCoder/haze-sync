# Component Contract: deployment

## Responsibility

Local deployment and compose scaffolding.

## Public interfaces

To be completed during the first component contract audit.

## Input contracts

To be completed.

## Output contracts

To be completed.

## Error contracts

Public errors must be safe and must not expose secrets, raw provider payloads, database URLs, local absolute paths, stack traces, bearer tokens, OAuth tokens, token hashes, or Idempotency-Key values.

## Persistence/runtime ownership

To be completed.

## Security and secrecy rules

- Do not commit secrets.
- Do not expose tokens or token hashes in public outputs.
- Do not expose local absolute paths or database URLs.
- Do not serialize raw provider payloads unless explicitly allowed by contract.

## Non-goals

To be completed.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

To be completed.

## Test obligations

To be completed during component planning.

## Contract change protocol

If implementation would require serious hacks, unsafe behavior, or cross-component changes, the worker must report `CONTRACT_CHANGE_REQUESTED` instead of silently broadening scope.
