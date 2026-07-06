# Documentation Unification Audit

## Purpose

This audit records the contradictions and duplication removed by the process documentation unification pass.

The goal is to make future development continue from one coherent model instead of mixing older wave-oriented notes, component docs baselines, and local project-source prompts.

## Canonical model after this pass

```text
protocol version: 2
development model: component-centric
worker scope: component
separate self-review phase: disabled
architect gate: optional/manual
required lifecycle: implementation -> clean-code-review -> CI -> fixer loop if needed
process/scaffold product-code changes: forbidden unless explicitly scoped
```

## Superseded model fragments

The following older fragments are no longer canonical:

1. Mandatory Architect review after every wave.
2. Separate self-review phase after implementation.
3. Worker scope limited to one function, one file, or one tiny task.
4. Workers archiving active prompt/report slots.
5. Treating local project-source prompts as repository docs by default.
6. Treating compose syntax validation as production deployment readiness.
7. Treating CI as live provider or production sync validation.
8. Allowing process/scaffold migrations to change product code by default.

These fragments may still exist in old local notes, old prompts, archived reports, or historical branch text. When encountered, use `docs-process/docs/development-model.md` as the current tracked model.

## Sources reviewed

Reviewed sources during this pass:

```text
implementation manifest project source
report template project source
GitHub connector/process notes project source
component/docs-process branch docs
component branch compare summaries
current README deployment/CI overview
current deploy/docker-compose.yml
current .github/workflows/ci.yml
current .github/workflows/component-ci.yml
```

The local project-source files were not copied verbatim into the repository. Their current rules were rewritten into repository-safe tracked docs.

## Component branch audit summary

All checked component branches were based on:

```text
main @ aabf74486d4d06a89136007bd17713f3c35478de
```

All checked branches had:

```text
behind_by: 0
```

Checked branches:

```text
component/common
component/core
component/api
component/storage
component/server
component/worktree
component/gdrive-adapter
component/obsidian-plugin
component/cli
component/deployment
component/github-ci
component/docs-process
```

## Findings

### 1. Component docs baselines exist

All component branches now have component-local documentation baselines or equivalent process docs.

Expected baseline files:

```text
component-contract.md
implementation-plan.md
dependency-map.md
decisions.md
implementation-log.md
```

### 2. Generic process rules were repeated too often

Many component docs repeat safety, lifecycle, ownership, and reporting ideas.

Resolution:

- Keep component-specific rules inside component docs.
- Move generic development model into `docs-process/docs/development-model.md`.
- Use `docs-process/docs/component-docs-guide.md` to prevent future duplication.

### 3. Active control slots need fan-in policy

Some component branches contain active control slots from completed worker tasks:

```text
control/prompt.md
control/report.md
```

Resolution:

- Treat active slots as process state, not static docs.
- Use `docs-process/docs/control-slots-and-reports.md` for the canonical lifecycle.
- Use `docs-process/docs/fan-in-and-merge-readiness.md` before merging those branches.

### 4. `component-ci.yml` exists in multiple branches

Multiple component branches add or modify:

```text
.github/workflows/component-ci.yml
```

Resolution:

- `github-ci` owns workflow policy.
- Choose one canonical workflow before broad fan-in.
- Do not let component docs merges accidentally overwrite workflow semantics.

### 5. Docs-only and mixed branches must be distinguished

Most component documentation branches are docs-only plus pre-existing `component-ci.yml`.

Known mixed branch risk:

```text
component/server includes a small product-code diff in crates/haze-sync-server/src/routes/health.rs
```

Resolution:

- Classify branches as docs-only, mixed, or product before fan-in.
- Mixed branches require explicit review.

### 6. System docs and process docs need separate roots

`docs/**` is suitable for product/system docs.

`docs-process/**` is suitable for process/development docs.

Resolution:

- Keep process docs under `docs-process/**`.
- Use `docs/**` for system architecture docs only when explicitly scoped.
- Do not import local handoff archives into either root by default.

## Unification changes made

This pass added:

```text
docs-process/README.md
docs-process/docs/development-model.md
docs-process/docs/component-docs-guide.md
docs-process/docs/control-slots-and-reports.md
docs-process/docs/fan-in-and-merge-readiness.md
docs-process/docs/unification-audit.md
```

This pass updates docs-process planning/log files to point future work at the canonical model.

## Remaining unresolved decisions

1. Canonical `component-ci.yml`
   - Owner: github-ci / Orchestrator.
   - Required before broad branch fan-in.

2. Active control slot cleanup for `core`, `api`, and `server`
   - Owner: Orchestrator.
   - Required before merging idle component docs.

3. Whether to merge component docs branches individually or through a fan-in branch
   - Owner: Orchestrator.

4. Whether system docs under `docs/**` should be merged before or after component docs
   - Owner: Orchestrator/Architect.

5. Whether report/status vocabulary should be enforced by CI later
   - Owner: docs-process + github-ci.

## Final rule

When documents conflict, use this precedence:

```text
1. active user instruction for the current task
2. docs-process/docs/development-model.md for tracked process rules
3. component-local contract for component-specific ownership
4. component-local implementation plan for phase ordering
5. dependency map for cross-component boundaries
6. implementation log and archived reports for historical context
```

Old prompts, old reports, local project-source files, and historical branch notes do not override the tracked canonical model unless the current prompt explicitly says so.
