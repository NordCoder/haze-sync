# Google Drive durable-state HTTP client

Phase `GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT` adds the adapter-owned client for the accepted private Server routes:

- `GET /v1/adapters/{adapter_id}/gdrive/state`
- `POST /v1/adapters/{adapter_id}/gdrive/state/commit`

Required identity configuration is `HAZE_GDRIVE_ADAPTER_ID`. It is validated with the shared `AdapterId` rules and is redacted from formatting. The existing adapter token remains the bearer credential.

The client applies bounded connect/request timeouts, disables redirects, bounds response bodies, validates private snapshots, protects pagination from loops and collection growth, and never retries compare-and-commit automatically. Mode gating happens before transport: reads require `core_reads`; commits require `durable_state_mutation`.

This phase does not add polling, reconciliation, Google provider operations, direct Storage access, status controls, CLI wiring, or Deployment configuration. Ordinary tests use a synthetic transport fake and no external network or credentials.
