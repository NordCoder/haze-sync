# W1-DEP-P7-BLOCKED — GDrive runtime and deployment-topology gate

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

DEP-P6 implementation and clean-code review are accepted with green CI. Storage STOR-P8 persistence repositories and GDrive GDA-P7 lifecycle are accepted.

GDA-P8 implementation and artifact-based CI correction are now green:

- GDA-P8 code-bearing SHA: 0120134b3c1a7b446b6c1c96953ce9abe4971d55
- Component CI run: 29127127665
- conclusion: success

GDA-P8C clean-code review is queued. Storage STOR-P9C is locally green, while the Server-owned feature-isolation correction is green and awaiting clean-code review. None of these phases defines the missing deployment topology.

## Blocked next phase

DEP-P7 requires an explicit accepted topology for:

- GDrive service process ownership and lifecycle;
- durable mapping, cursor, and delete-candidate repository wiring;
- OAuth/token secret loading, rotation, and redaction;
- Server/API transport, health, and readiness exposure;
- retry/backoff and safe shutdown behavior.

Deployment must not invent direct database ownership, an adapter daemon contract, or secret-handling policy.

## Unblock condition

Green GDA-P8C acceptance plus an explicit GDrive/Storage/Server runtime-config fan-in contract defining deployment ownership and topology. Until then, do not launch a worker from this hold notice.
