# W1-DEP-P7-BLOCKED — GDrive service/config fan-in dependency gate

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted state

DEP-P6 implementation and clean-code review are accepted. Final docs/config-bearing CI is green.

- code_bearing_sha: 66267862e6de63b405ded83ace54b458ffeb1306
- workflow: Component CI
- workflow_run_id: 29082102227
- run_number: 979
- conclusion: success

Storage STOR-P8 mapping and Worktree-state repositories are now clean-code/CI accepted. GDrive GDA-P7 implementation and CI correction are complete, and GDA-P7C is queued for clean-code review.

## Blocked next phase

The next plan phase is DEP-P7: GDrive adapter service and OAuth secret layout.

Deployment still lacks an accepted concrete service topology and runtime/config contract for:

- durable mapping and cursor repository wiring;
- GDrive process ownership and lifecycle;
- OAuth/token secret loading and rotation boundaries;
- Server/API transport and health/readiness exposure;
- retry/backoff and safe shutdown behavior.

Deployment must not choose direct DB ownership, invent an adapter daemon contract, or define secret handling before the owning component boundaries are accepted.

## Unblock condition

Unblock after GDA-P7C is accepted and either:

1. a later GDrive runtime/config phase plus required Server/Storage fan-in contracts are accepted with green CI; or
2. a dedicated cross-component prompt explicitly defines the deployment topology, owners, secrets, persistence, lifecycle, and health boundaries.

Until then, do not run a worker for this component and do not modify deployment product files.
