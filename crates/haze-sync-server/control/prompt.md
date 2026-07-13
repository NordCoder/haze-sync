# W1-SRV-P7B4-WAIT-WT-P11 — Await Worktree hosted-runtime contract extension

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: none
Phase: SRV-P7B4-WAIT-WT-P11

This is a hold notice, not an executable worker prompt.

SRV-P7B4 is blocked by an accepted Worktree owner-contract gap documented at Server report commit `03e15367e1363bc4718581ca2c8993be52c2231e`, report blob `7172be029d2480eb151be2151f36718a985d089d`.

Required Worktree owner deliverables:

- production path-free `WorktreeWatcher` implementation or accepted Worktree-owned watcher factory/channel boundary with lifecycle semantics;
- scheduler-accounted manual-cycle entrypoint on `WorktreeRuntimeService` that preserves counters, last-cycle state, lifecycle/cancellation and no-overlap semantics;
- typed coarse busy/cancelling/shutdown outcomes;
- focused tests, green Component CI and mandatory clean review.

Do not launch a Server worker while this hold is active. Do not implement watcher or duplicate manual scheduler accounting in Server.

Reactivation condition:

1. Worktree owner phase is `CLEAN_ACCEPT` on an exact code-bearing SHA;
2. Orchestrator explicitly synchronizes that exact Worktree product snapshot into `component/server`;
3. Server integration CI and clean review pass;
4. Orchestrator restores an executable SRV-P7B4 implementation prompt.

API-P8 and SRV-P7B5 remain blocked.
