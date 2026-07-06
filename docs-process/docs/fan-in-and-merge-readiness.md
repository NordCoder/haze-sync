# Fan-in and Merge Readiness

## Purpose

Fan-in is the process of bringing completed component branches into an integration or target branch without losing component boundaries, process state, or safety evidence.

This guide defines the default fan-in policy for Haze Sync component branches.

## Merge readiness is not implementation acceptance

A worker report can self-accept an implementation, but merge readiness requires more:

```text
component contract satisfied
clean-code review completed when required
CI status understood
control files handled
branch diff reviewed
cross-component conflicts resolved
safety/secrecy checked
```

Passing CI is not production readiness. It is repository validation evidence.

## Fan-in source branches

Component work uses `component/*` branches.

Process/documentation migrations use `process/*` branches.

A component branch may contain multiple passes:

```text
implementation
clean-code-review
fixer
architect/documentation updates
```

Before fan-in, review the branch as a whole, not only the latest commit.

## Required branch facts

For every fan-in candidate, collect:

```text
branch name
base branch
merge base SHA
ahead_by
behind_by
changed files
whether product code changed
whether control active slots exist
whether workflow files changed
whether CI was observed
```

If `behind_by` is non-zero, rebase/merge strategy requires Orchestrator decision. Do not silently assume it is safe.

## Component docs fan-in checklist

For docs/planning branches, verify:

```text
component-contract.md exists and is component-specific
implementation-plan.md has current state, target state, phases, risks, deferred work
dependency-map.md records upstream/downstream and forbidden dependencies
decisions.md records durable decisions
implementation-log.md records the planning pass
current-state claims match actual repository code
no active control prompt/report was accidentally created
no product code changed unless explicitly scoped
```

## Product-code fan-in checklist

For product branches, verify:

```text
allowed component scope was respected
component contract was read and satisfied
implementation report exists
clean-code review exists or is intentionally deferred by Orchestrator
checks are honestly reported
CI is green or failure is triaged
no forbidden data or generated artifacts were committed
```

## Active control slot policy

Active control files are process state:

```text
control/prompt.md
control/report.md
```

Default fan-in policy:

1. If the active prompt/report pair is complete and safe, archive it under `control/log/` before final merge.
2. If the active prompt/report pair is stale, remove active slots after preserving useful history in `control/log/` if appropriate.
3. Keep `control/state.md` accurate.
4. Do not merge stale active prompts as static documentation.

This policy is especially important for branches that already contain completed implementation-worker reports.

## Workflow file duplicates

If many component branches add or modify the same workflow file, choose one canonical workflow before fan-in.

Current known risk:

```text
.github/workflows/component-ci.yml
```

Multiple component branches may contain similar but not identical copies. The github-ci component owns workflow policy. Resolve the canonical file through `component/github-ci` or a process fan-in branch before merging many component branches.

## Product-code drift in docs branches

Docs-only passes must not modify product code.

If a docs branch contains product-code changes, classify the branch accurately:

```text
docs-only: only docs/process files changed
mixed: docs plus product/CI/deploy files changed
product: product/runtime behavior changed
```

Mixed branches require explicit fan-in review.

## Suggested fan-in order

Default order for docs/process normalization:

```text
1. github-ci or process branch for canonical workflow policy
2. docs-process canonical guides
3. common docs
4. storage/core/api docs according to dependency needs
5. server docs
6. worktree/gdrive/obsidian/cli/deployment docs
7. final repository docs index update
```

Default order for product implementation fan-in depends on dependency graph and CI state. Do not use docs-only order for product-code merges.

## Fan-in report

A fan-in report should include:

```text
branches reviewed
branches merged or not merged
canonical workflow decision
active control-slot decision
product-code changes included
CI status observed
conflicts resolved
remaining blockers
next recommended agent
```

## When to invoke Architect

Invoke Architect manually when fan-in finds:

- conflicting component contracts;
- unclear dependency ownership;
- API/Core/Storage semantics mismatch;
- control-slot policy disagreement;
- workflow policy that affects required checks;
- product-code changes hidden in docs/process branches;
- unsafe deletion, provider, credential, or deployment behavior.

Architect review is optional/manual, not automatic after every wave.

## No PR or merge by default

Workers and docs-process passes do not open PRs or merge branches unless explicitly instructed.

Orchestrator or the user decides when to open PRs, merge, or archive branches.
