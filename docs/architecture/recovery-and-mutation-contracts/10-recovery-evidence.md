# Recovery and Mutation Contracts — Recovery Acceptance Evidence

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 11. Recovery acceptance evidence

Later R6-4 recovery acceptance MUST retain a durable evidence bundle containing at least:

```text
operation IDs, job versions, helper attempt IDs, and kind mappings
executor IDs and monotonically allocated executor fences
lease heartbeat/expiry history without raw lease tokens/digests in public output
global/scoped execution slot IDs and slot-version history
execution scope digests
source and target environment identities
backup and manifest IDs/digests
artifact SHA-256 values, sizes, and readability results
database table counts
object-store blob count and byte count
Worktree file count and byte count when applicable
deterministic selected blob/file hashes
highest operation-log sequence
product/control schema and applied migration state
source, target, rollout, and rollback release-set identities
maintenance generation
adapter inventory generation and complete adapter_instances snapshot
all expected/applied adapter-control and maintenance generations
quiescence evidence ID/digest and per-adapter drain/fence proof
Obsidian authoritative mutation-gate evidence
doctor and preflight results
restore/rollout phase checkpoints and externally fenced step identities
rollback decision checkpoint
operator confirmation audit IDs and bound job version/generations
external recovery-control-envelope ID/version/state history
live target control-plane before/after digests and excluded record families
proof that source control-plane rows remained inactive
proof that stale executors could not checkpoint/terminate after takeover or outage
proof that slot remained held/blocked across uncertain or PostgreSQL-unavailable windows
final slot release, envelope closure, and resumption decision
```

Evidence MUST distinguish `not run`, `skipped`, `failed`, `blocked_uncertain`, and `passed`. Missing evidence MUST NOT be represented as success.
