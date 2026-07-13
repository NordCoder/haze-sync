REPORT_TYPE:
FIX

STATUS:
FIX_BLOCKED_BY_CONTRACT

AGENT:
role: fixer-worker
agent_execution_id: server-srv-p7b5-manual-availability-fix-20260714-contract-blocker
chat_name: server — W1 SRV-P7B5 Manual Availability Fix

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-P7B5-MANUAL-AVAILABILITY-FIX

REVIEWED_CANDIDATE:
code_bearing_sha: 78f4e4525327ff03fa1af1e8387e9e3ea07091d6
functional_review_report_commit: 8082bc6fdd65053f66945fba5ed9047b369442e1
functional_review_report_blob: 2f43f88adb950f59100aad80d860b268a54f4f73
review_status: CLEAN_NEEDS_FIX

SUMMARY:
No Server-owned product workaround was committed. A correct race-safe manual-availability projection cannot be derived from the currently accepted Worktree contract without approximating or duplicating the authoritative manual gate. The active prompt explicitly requires FIX_BLOCKED_BY_CONTRACT in this case.

CURRENT_ACCEPTED_CONTRACT:
- WorktreeRuntimeManualHandle::submit returns Accepted(ticket), Busy, NotStarted, Cancelling or Shutdown.
- The accepted handle owns an internal lifecycle/busy gate but exposes no read-only status or generation.
- WorktreeRuntimeManualTicket exposes only try_result() to the ticket holder.
- WorktreeHostedRuntime::poll returns Manual(outcome), Automatic(result) or Idle, but Manual(outcome) carries no request identity/generation.
- The accepted runtime may release its internal busy gate immediately before poll returns.

SERVER_ONLY_PROTOCOLS_EVALUATED:
1. Poll-based Busy/Available mirror.
   Result: rejected. It produces false Busy during idle DryRun polling and can overwrite a newer accepted request with Available.

2. Submission generation plus clear-on-Manual completion.
   Proposed shape: assign a monotonically increasing generation on typed Accepted, capture an active generation around poll, and compare-exchange that generation to Available when Manual completion returns.
   Result: insufficient. A request can be accepted after the Server captures the active generation but before WorktreeHostedRuntime::poll executes receiver.try_recv(). The poll may process that newer request, yet the returned Manual outcome contains no generation. Clearing the captured generation either fails to clear the completed request or risks clearing a later request, depending on capture timing.

3. Clear-on-ticket-observation wrapper.
   Result: insufficient. Completion would remain Busy until the caller polls the ticket, and dropping the ticket cannot safely distinguish pending from terminal completion. This does not represent actual accepted request completion.

4. Server-side synchronization around submit and poll.
   Result: rejected. It would create a second Server-owned gate, alter accepted typed Busy/non-blocking submission semantics, or hold synchronization across an awaited cycle. This violates Worktree ownership and no-duplication requirements.

PRECISE_CONTRACT_GAP:
The Server lacks one of the following authoritative read-only facts:
- current Worktree manual gate lifecycle/busy state; or
- stable request generation/identity included in both Accepted submission and Manual completion.
Without either, Server cannot determine whether Busy belongs to the request just completed, a newer accepted request, or no manual request at all.

MINIMAL_OWNER_EXTENSION_REQUIRED:
Preferred minimal extension:
- add a cloneable read-only WorktreeRuntimeManualStatusHandle or status method sharing the existing accepted Gate;
- expose only coarse typed fields:
  - lifecycle: Created | Running | Cancelling | Shutdown;
  - busy: bool;
- reads must be lock-free/passive, perform no submission, and expose no request payload, path, token, cursor or backend details;
- Server then maps lifecycle/busy directly into its existing manual availability categories and removes its AtomicU8 mirror entirely.

Acceptable alternative:
- add an opaque monotonically increasing manual request generation to Accepted(ticket) and return the same generation with WorktreeHostedRuntimePoll::Manual;
- generation must be assigned and cleared by the existing Worktree gate owner;
- no payload or backend detail should be exposed.

OWNER_SCOPE:
component: worktree
likely_paths:
- crates/haze-sync-worktree/src/hosted_runtime.rs
- focused Worktree tests
required_semantics:
- authoritative read-only gate projection
- no change to current submit Busy behavior
- no new task/poller/runtime
- cancellation/shutdown state remains authoritative
- exact-SHA Worktree CI and clean review before Server fan-in

SERVER_FOLLOW_UP_AFTER_OWNER_ACCEPTANCE:
- exact-SHA fan-in of the accepted Worktree owner extension;
- remove Server manual_gate AtomicU8 and all poll-based toggles;
- map accepted lifecycle/busy status directly in snapshot();
- add deterministic tests for idle DryRun Available, accepted Busy lifetime, concurrent newer submission safety, Busy-is-Ready, and lifecycle overrides;
- run authoritative DB-capable Server Component CI and focused clean review.

CODE_CHANGES:
none

CI:
No new code-bearing SHA exists in this fixer phase. The reviewed candidate remains green in Component CI run 29283227887, but its known manual-availability defect remains unresolved.

PROTECTED_SCOPE:
accepted_worktree_modified: no
accepted_storage_modified: no
server_product_modified: no
migrations_or_schema_modified: no
core_api_cli_deployment_modified: no
public_routes_dtos_openapi_or_readiness_modified: no
workflows_modified: no
sibling_branches_modified: no
pr_merged_or_draft_state_changed: no
history_rewritten: no

BLOCKERS:
- accepted Worktree manual handle has no authoritative read-only lifecycle/busy status
- accepted Manual completion has no request identity/generation
- every Server-only completion projection retains an uncloseable acceptance/dequeue/completion race or duplicates gate ownership

NEXT_RECOMMENDED_AGENT:
orchestrator

NEXT_GATE:
Route a minimal Worktree owner contract extension. API-P8 remains blocked. Do not resume SRV-P7B5 until the accepted Worktree extension is fan-in integrated and reviewed.

FINAL_VERDICT:
FIX_BLOCKED_BY_CONTRACT. The requested race-safe authoritative projection is not implementable from the accepted Worktree API without an approximate second gate. The minimal safe resolution is a read-only Worktree manual lifecycle/busy status handle, or an owner-assigned request generation carried through Accepted and Manual completion.

PUSHED:
yes
