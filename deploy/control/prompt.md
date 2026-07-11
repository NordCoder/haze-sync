# W1-DEP-P7-BLOCKED-BY-RUNTIME-TOPOLOGY — Deployment fan-in gate

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

DEP-P6 implementation and clean-code review are accepted with green CI.

The following dependencies are now accepted:

- GDrive Adapter component-local lifecycle through GDA-P8C and post-fix CI;
- Storage component-local lifecycle;
- Server-owned Storage production feature isolation and its clean review.

The missing work is no longer a GDA-P8C review gate. It is the explicit cross-component runtime and deployment topology.

## Blocked next phase

DEP-P7 requires accepted ownership and configuration contracts for:

- GDrive service process ownership, startup ordering, retry/backoff, and shutdown;
- durable mapping, cursor, and delete-candidate repository wiring;
- OAuth/token secret loading, rotation, persistence, and redaction;
- Server/API transport, authentication, health, readiness, and operator surfaces;
- Worktree/GDrive runtime composition and bootstrap rollout order;
- backup/restore and rollback boundaries for metadata and object storage.

Deployment must not invent direct database ownership, adapter daemon semantics, OAuth policy, or Server runtime contracts.

## Unblock condition

An explicit accepted GDrive/Storage/Server/API runtime-config and lifecycle fan-in contract defining deployment ownership and topology. Until then, do not launch a worker from this hold notice.
