# W1-SRV-P7B5-WAIT-WT-MANUAL-STATUS — Hold for Worktree owner extension

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: orchestrator-hold
Phase: SRV-P7B5-WAIT-WT-MANUAL-STATUS

Do not implement, merge, change draft state, rewrite history, or modify product files.

SRV-P7B5 remains blocked because the accepted Worktree manual contract exposes neither an authoritative read-only lifecycle/busy status nor a stable request identity carried through submission and completion.

Accepted Server candidate remains:

- code-bearing SHA `78f4e4525327ff03fa1af1e8387e9e3ea07091d6`;
- DB-capable Component CI run `29283227887`, number `1901`, success;
- readiness/status semantics accepted except manual availability projection.

Wait for:

1. minimal Worktree owner extension;
2. exact-SHA Worktree CI and clean review;
3. exact-SHA fan-in to Server;
4. focused Server removal of the approximate manual mirror and final SRV-P7B5 verification.

API-P8 remains blocked.
