# W1-DEP-P7-BLOCKED — GDrive runtime and deployment-topology gate

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

DEP-P6 implementation and clean-code review are accepted with green CI. Storage STOR-P8 persistence repositories and GDrive GDA-P7 export planning/clean-code/fixer lifecycle are accepted with green CI.

GDA-P8 delete-guard implementation is now queued, but it does not itself define service process ownership, OAuth secret handling, or Server/API runtime composition.

## Blocked next phase

DEP-P7 requires an explicit accepted topology for:

- GDrive service process ownership and lifecycle;
- durable mapping and cursor repository wiring;
- OAuth/token secret loading, rotation, and redaction;
- Server/API transport, health, and readiness exposure;
- retry/backoff and safe shutdown behavior.

Deployment must not invent direct database ownership, an adapter daemon contract, or secret-handling policy.

## Unblock condition

An explicit GDrive/Storage/Server runtime-config fan-in contract defines the deployment topology with accepted ownership and green evidence. Until then, do not launch a worker from this hold notice.
