PROMPT_ID: orch-plan-20260719-01
CHAT_KEY: orchestrator
ROLE: orchestrator
COMPONENT: none
WORK_ITEM: none
PLAN_SOURCE: haze-sync-v1-completion-plan.md
PLAN_SECTIONS:
- full plan
- §4.2 API-R0
- §4.3 STOR-R1
- §4.6 CLI-R1
- §4.8 GDA-R2
- §6 Wave R1
- §8 First dispatch batch

# Summary

Read the authoritative completion plan and current GitHub evidence. Prepared the fixed first Wave R1 batch as four independent workflow-minimal targets.

API-R0 was assigned as a focused clean-code review rather than duplicate implementation because the implementation and successful exact-SHA CI evidence already exist; that review is the remaining formal gate. GDA-R2, CLI-R1, and STOR-R1 remain implementation assignments.

No product code, `main`, Dispatcher state, legacy control state, or sibling product branch was modified.

# Prepared targets

- `5 api` / `5-api-orch-plan-20260719-01` / `clean-code-reviewer` / `API-R0` / `component/api` @ `09a4495ed139a0076260c6ba429d616b5d6df836` / prompt commit `df6ed2b511e07a2f8059371aec559b0371f7b550`.
- `2 gdrive-adapter` / `2-gdrive-adapter-orch-plan-20260719-01` / `implementation-worker` / `GDA-R2` / `component/gdrive-adapter` @ `7f85f1cc931b668e766c963da064cd857f88418c` / prompt commit `cae1b7a9b46575013d160304a364723474d2d908`.
- `8 cli` / `8-cli-orch-plan-20260719-01` / `implementation-worker` / `CLI-R1` / `component/cli` @ `b174f333219d150704af4d717d78c7ede8700211` / prompt commit `1757cdbb16ac2119472d1cb5ecaa212c5701cce9`.
- `6 storage` / `6-storage-orch-plan-20260719-01` / `implementation-worker` / `STOR-R1` / `component/storage` @ `bfcca9664157dffef8f3162362cbdc1770368594` / prompt commit `3eab92d3769ce36e7eeb1eb17d14f4364517f982`.

# Changes

- Added only `.agentic/prompt.md` on each of the four new target control branches.
- Product changes: none.
- Dispatcher changes: none.

# Repository evidence

- main baseline: `c1e69a664388b0cba028170e8398b9088218957d`;
- Orchestrator input control commit: `37b81d9cee66b498b33e4218254cd663e3d21388`;
- API verified CI candidate: `dba43751521c32aca53729c1c8dbddf2e7d8fbfb`, successful run `29585502722`;
- all four current product heads were reread and bound exactly in their prompts;
- each target control branch is exactly one commit ahead of main and contains only `.agentic/prompt.md`.

# Checks

- current Orchestrator prompt identity and blob reread before planning and before terminal publication: passed;
- completion-plan first-batch identity, dependencies, exclusions, and target order: passed;
- every selected work item remains incomplete or has a remaining formal gate: passed;
- every target prompt contains workflow identity, deterministic prompt ID, exact chat key, role, work item, plan source/sections, product branch/input commit, bounded scope, checks, and terminal rules: passed;
- every target prompt reread after creation: passed;
- every target branch compared with main and verified to contain only the prompt: passed;
- browser triggers: not sent;
- product checks: not run because no product work was performed.

# Remaining work

Dispatcher should deliver the four exact targets in the matching done marker. The next Orchestrator turn should consume their matching terminal reports after all four complete.

# Blockers and uncertainty

- none.

# Suggested next action

Dispatch only the four prepared workflow-minimal targets listed in `.agentic/done.json`.