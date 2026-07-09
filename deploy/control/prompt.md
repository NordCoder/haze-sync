# W1-DEP-P3-BLOCKED — Deployment waiting for Server dependency

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none

## Status

DEP-P3 is blocked by dependency.

The latest deployment report concluded BLOCKED_BY_DEPENDENCY: Deployment must not invent Server startup, listener/config, readiness/liveness, migration, container, or service behavior on Server's behalf.

Server SRV-P3 is accepted in component/server, but Deployment must wait until the Server deployment-relevant surface is available through main/fan-in or an explicit Orchestrator prompt authorizes a cross-component dependency basis.

## Worker behavior

No worker should execute deployment implementation work from this prompt.

If a deployment worker is asked to continue while control/state.md is not PROMPT_READY, it should stop and report that deployment has no active prompt ready.

## Next condition

Orchestrator may unblock DEP-P3 only after the Server startup/config/readiness/migration surface is accepted in the dependency baseline used by Deployment.
