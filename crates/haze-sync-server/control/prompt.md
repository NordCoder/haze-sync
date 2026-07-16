# W1-SRV-GDA-P1-ACCEPTED-HOLD

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: orchestrator-hold
Phase: SRV-GDA-P1-ACCEPTED-HOLD

This is a hold notice, not an executable worker prompt.

Accepted Server GDrive route/application candidate:
- code-bearing SHA: `c023b83e1e6f502e7d2261acccb871dd5588edf1`;
- clean-review report blob: `1c223ebda33a1550fa8cbf38a15079ef7810c63d`;
- DB-capable Component CI run: `29494321838`, run number `2045`, success.

Accepted surface:
- authenticated private adapter state GET and sanitized Admin GET;
- matching GDrive adapter-only compare-and-commit POST;
- adapter identity/type verification before Storage access;
- caller-owned PostgreSQL transactions and rollback-before-response mapping;
- committed/replayed commit behavior;
- typed stale, cursor gap/regression, invalid cursor state, mapping, idempotency, validation and internal outcomes;
- PostgreSQL replay, concurrency, rollback, isolation and secrecy evidence;
- exact accepted API and Storage dependency identity.

Downstream authorization:
- `GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT` may consume these exact public routes and semantics.

Do not implement, merge, change draft state, rewrite history, modify sibling branches, workflows or product files. Wait for explicit Orchestrator assignment.
