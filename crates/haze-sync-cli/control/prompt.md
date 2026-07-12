# W1-CLI-P6A-BLOCKED-BY-API-SERVER — Await accepted status and sync-once contracts

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: none
Phase: CLI-P6A-BLOCKED-BY-API-SERVER

This is a hold notice, not an executable worker prompt.

The existing CLI component-local lifecycle through CLI-P5 is accepted with green Component CI run `29084418994`, run number `1047`.

The next planned CLI phase is `CLI-P6A — Worktree status and explicit sync-once operator commands`.

CLI-P6A must not begin until all of the following are clean-accepted and synchronized:

1. Server exact-SHA Worktree/Storage fan-in;
2. `SRV-P7B3` bounded Worktree executor;
3. `SRV-P7B4` hosted Worktree runtime;
4. `API-P8` passive Worktree runtime status/operator contract;
5. `SRV-P7B5` safe status/readiness and optional manual one-cycle endpoint.

WT-P10, STOR-P10 and SRV-P7B2 are already accepted; the current blocker is the downstream runtime and public operator surface, not Worktree clean review.

When unblocked, CLI may consume only accepted Server/API contracts. It must not implement local runtime hosting, direct database/filesystem mutation, provider calls, hidden daemon ownership or destructive repair.

Before future CLI-P6A work, synchronize `component/cli` with current `main` through an explicit Orchestrator sync.

Do not launch a CLI worker from this notice.