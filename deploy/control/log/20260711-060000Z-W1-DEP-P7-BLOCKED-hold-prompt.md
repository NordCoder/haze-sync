# W1-DEP-P7-BLOCKED — GDrive runtime and deployment-topology gate

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

DEP-P6 implementation and clean-code review are accepted with green CI. Storage STOR-P8 persistence repositories and GDrive GDA-P7 lifecycle are accepted.

GDA-P8 delete-guard implementation is complete, but its final source/test run is formally red and routed to `FIX-GDA-P8-CI` using artifact `8240045016`. Storage STOR-P9C is locally green but waiting for a Server-owned production feature-isolation correction.

Neither correction defines the missing service process, OAuth-secret, or Server/API composition topology.

## Blocked next phase

DEP-P7 requires an explicit accepted topology for:

- GDrive service process ownership and lifecycle;
- durable mapping, cursor, and delete-candidate repository wiring;
- OAuth/token secret loading, rotation, and redaction;
- Server/API transport, health, and readiness exposure;
- retry/backoff and safe shutdown behavior.

Deployment must not invent direct database ownership, an adapter daemon contract, or secret-handling policy.

## Unblock condition

Green GDA-P8 lifecycle plus an explicit GDrive/Storage/Server runtime-config fan-in contract defining deployment ownership and topology. Until then, do not launch a worker from this hold notice.
