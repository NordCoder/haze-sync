# W1-DEP-P5A-BLOCKED-BY-SRV-P7B5 — Await accepted Worktree runtime topology

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: none
Phase: DEP-P5A-BLOCKED-BY-SRV-P7B5

This is a hold notice, not an executable worker prompt.

The existing Deployment component-local lifecycle through DEP-P6C is accepted with green Component CI run `29082102227`, run number `979`.

The next architecture-planned Deployment phase is `DEP-P5A — Worktree runtime/config fan-in`.

DEP-P5A must not begin until:

1. STOR-P10 remains clean-accepted and its migration/runtime-state contract is synchronized into the Server integration line;
2. Server exact-SHA Worktree/Storage fan-in is clean-accepted;
3. `SRV-P7B3` bounded Worktree executor is clean-accepted;
4. `SRV-P7B4` hosted runtime/config/startup/shutdown contract is clean-accepted;
5. `API-P8` and `SRV-P7B5` safe status/readiness/operator surfaces are clean-accepted;
6. migration execution policy and operational ownership are explicit.

When unblocked, Deployment may change only `deploy/**` and safe placeholder alignment such as `.env.example`. It will own paths, permissions, service lifecycle, migration/backup/rollback procedure and staged rollout documentation. It must not invent product runtime behavior, OAuth policy, direct database ownership, hidden migrations, real credentials or automatic bidirectional enablement.

GDrive and Obsidian component-local implementations are already accepted, but their future real runtime/release integration does not unblock Worktree Deployment topology ahead of Server contracts.

Before future DEP-P5A work, synchronize `component/deployment` with current `main` through an explicit Orchestrator sync.

Do not launch a Deployment worker from this notice.