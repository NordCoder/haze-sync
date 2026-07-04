# Current State

This repository has completed W3 conflict/delete/admin safety fan-in and is migrating to a component-centric development protocol.

## Known post-W3 themes

- Core contains pure revision/conflict/delete/idempotency/doctor primitives.
- API contains passive route-contract helpers.
- Storage contains passive repository helpers and row models.
- Server wires runtime behavior and currently contains special route intercept/fallback composition for W3 routes.
- Adapter runtimes remain future work.
- Architect gate is optional/manual for the next development stage.
