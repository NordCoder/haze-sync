# Recovery and Mutation Contracts

Status: normative V1 contract  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Owned scope: coordinated backup/restore, upgrade/rollback, and transactional `accept_conflict`

## Authority and ordered document set

This file is the normative entry point. The complete contract is partitioned into the following ordered files. Every file has equal normative authority and MUST be reviewed and implemented as part of the same contract:

1. [`01-scope-and-compatibility.md`](recovery-and-mutation-contracts/01-scope-and-compatibility.md) — logical sections 1–2.
2. [`02-maintenance-and-jobs.md`](recovery-and-mutation-contracts/02-maintenance-and-jobs.md) — logical sections 3.1–3.2.
3. [`03-idempotency-credentials-audit.md`](recovery-and-mutation-contracts/03-idempotency-credentials-audit.md) — logical sections 3.3–3.5.
4. [`04-execution-ownership.md`](recovery-and-mutation-contracts/04-execution-ownership.md) — logical section 4.
5. [`04-helper-contracts.md`](recovery-and-mutation-contracts/04-helper-contracts.md) — logical section 5.
6. [`05-quiescence.md`](recovery-and-mutation-contracts/05-quiescence.md) — logical section 6.
7. [`06-backup.md`](recovery-and-mutation-contracts/06-backup.md) — logical section 7.
8. [`07-restore-authority.md`](recovery-and-mutation-contracts/07-restore-authority.md) — logical sections 8.1–8.2.
9. [`08-restore-execution.md`](recovery-and-mutation-contracts/08-restore-execution.md) — logical sections 8.3–8.6.
10. [`09-upgrade-and-rollback.md`](recovery-and-mutation-contracts/09-upgrade-and-rollback.md) — logical sections 9–10.
11. [`10-recovery-evidence.md`](recovery-and-mutation-contracts/10-recovery-evidence.md) — logical section 11.
12. [`11-accept-conflict-transaction.md`](recovery-and-mutation-contracts/11-accept-conflict-transaction.md) — logical sections 12.1–12.7.
13. [`12-accept-conflict-outcomes.md`](recovery-and-mutation-contracts/12-accept-conflict-outcomes.md) — logical sections 12.8–12.11.
14. [`13-downstream-and-invariants.md`](recovery-and-mutation-contracts/13-downstream-and-invariants.md) — logical sections 13–14.

Section numbering remains global across the ordered files. A child file MUST NOT be interpreted in isolation. Recovery request/result/manifest/envelope records are bounded evidence for the canonical `haze-sync.operational-job.v1`; they do not create parallel job, CAS, confirmation, idempotency, executor, lease, slot, or audit authority.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative throughout the document set.
