# W1-DEP-P7-BLOCKED — GDrive runtime and deployment-topology gate

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

DEP-P6 implementation and clean-code review are accepted with green CI. Storage STOR-P8 mapping and Worktree-state repositories are accepted.

GDA-P7 implementation and GDA-P7C clean-code work are present, but the final GDA-P7C source/test head is formally red and has been routed to `FIX-GDA-P7C-CI` using artifact `8236735946`.

STOR-P9 test-support implementation is also present but is in its own CI-fixer lifecycle; it does not define the missing runtime topology.

## Blocked next phase

DEP-P7 requires an accepted concrete boundary for:

- GDrive service process ownership and lifecycle;
- durable mapping and cursor repository wiring;
- OAuth/token secret loading, rotation, and redaction;
- Server/API transport, health, and readiness exposure;
- retry/backoff and safe shutdown behavior.

Deployment must not invent direct database ownership, an adapter daemon contract, or secret handling policy.

## Unblock condition

Unblock only after GDA-P7C completes its fixer lifecycle with green CI and an explicit runtime/config/service fan-in contract defines the deployment topology. Until then, do not launch a worker from this hold notice.
