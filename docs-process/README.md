# Haze Sync Development Process

This directory is the canonical tracked documentation layer for Haze Sync development process and documentation conventions.

It exists to remove duplication between component-local docs, old wave-oriented planning notes, local project-source prompts, and branch-specific reports.

## Canonical model

The active model is:

```text
component-centric development
implementation -> clean-code-review -> CI -> fixer loop if needed
optional/manual Architect review
```

Older wording that implies mandatory Architect review after every wave, separate self-review phases, or function/file-level worker scope is obsolete for current development.

## Documents

```text
docs/development-model.md
  Canonical roles, lifecycle, branch policy, source order, status vocabulary,
  check honesty, secrecy rules, and contract-change protocol.

docs/component-docs-guide.md
  How to write component-local contracts, implementation plans, dependency maps,
  decisions, and implementation logs without duplicating generic process rules.

docs/control-slots-and-reports.md
  Active control slot lifecycle and report format semantics.

docs/fan-in-and-merge-readiness.md
  How to fan in component branches, handle active control slots, workflow duplicates,
  CI state, and merge-readiness evidence.

docs/unification-audit.md
  What was unified, which older model fragments are superseded, and what remains
  unresolved.
```

## Relationship to component docs

Component docs remain local to component roots and remain the source of truth for component-specific responsibilities.

Process docs define conventions. They do not override component ownership, change product behavior, create active prompts, write reports, open PRs, merge branches, or deploy services.

## Relationship to project-source files

Local project-source manifests, prompt templates, reports, and handoff notes are not repository artifacts by default.

When process rules need to become tracked documentation, they must be rewritten into repository-safe docs rather than copied verbatim.

## Safety

Tracked process docs must not include real secrets, production `.env` values, OAuth tokens, bearer tokens, token hashes, provider payloads, raw logs, private scratch, or local absolute user paths.
