# Control Slots and Reports

## Purpose

Control folders are process state for active component work.

They are separate from component documentation. Component docs describe the component; control files assign and report work.

## Standard control folder

```text
control/README.md
control/state.md
control/log/.gitkeep
```

Active files:

```text
control/prompt.md
control/report.md
```

Archived files:

```text
control/log/YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-prompt.md
control/log/YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-report.md
```

## Active slot rules

- A component has at most one active prompt.
- A component has at most one active report for that prompt.
- Orchestrator creates `control/prompt.md`.
- Implementation Worker, Clean-Code Reviewer, or Fixer writes `control/report.md` when assigned.
- Orchestrator archives active prompt/report files before creating a new prompt.
- Workers do not archive prompt/report files unless explicitly assigned as Orchestrator.
- Workers do not read old archived prompts/reports unless the active prompt explicitly references them.

## Idle component state

An idle component should not contain active `control/prompt.md` or active `control/report.md`.

The idle state is:

```text
control/README.md
control/state.md
control/log/.gitkeep
```

plus any archived logs.

If a component branch contains active prompt/report files, that branch is carrying process state. Before fan-in, Orchestrator must decide whether to keep, archive, or remove those active files.

## Worker source order

When a worker is assigned, it reads:

```text
1. canonical process model, when referenced
2. component contract
3. component implementation plan
4. component implementation log
5. component dependency map
6. active control prompt
7. current repository code
```

The active prompt may point to additional sources. Otherwise, old prompts and reports are context only for Orchestrator.

## Report file

Workers write the report to:

```text
<component>/control/report.md
```

unless the active prompt explicitly assigns another report path.

A report must be a factual record of what changed, what was checked, what was not checked, and what remains blocked.

## Report fields

Canonical report shape:

```text
REPORT_TYPE:
STATUS:
AGENT:
COMPONENT:
WAVE:
SUMMARY:
CHANGED_FILES:
BRANCH_AND_CONTROL:
SCOPE:
CONTRACT:
IMPLEMENTATION_OR_REVIEW:
TESTS_AND_CHECKS:
SAFETY_AND_SECRECY:
ISSUES_FOUND:
BLOCKERS:
NEXT_RECOMMENDED_AGENT:
FINAL_VERDICT:
PUSHED:
```

The report may add clarifying subsections, but it should preserve these top-level fields when following the standard template.

## Report type

Allowed values:

```text
IMPLEMENTATION
CLEAN_CODE_REVIEW
FIX
ARCHITECT_REVIEW
BLOCKED
```

## Status

Use the canonical status vocabulary from `docs-process/docs/development-model.md`.

Do not invent new statuses in worker reports unless the active prompt explicitly extends the vocabulary.

## Branch and control honesty

Reports must say:

```text
current_branch
base_branch
base_sha
head_sha if known
default_branch_modified
sibling_branch_modified
control_prompt_read
control_report_written
control_files_archived_by_worker
```

If working through GitHub connector only and the exact head SHA is not observed, say so.

## Scope honesty

Reports must say:

```text
allowed_files_only
scope_expansion_used
scope_expansion_rationale
cross_component_changes
forbidden_files_touched
```

Component-internal expansion is allowed when needed, but it must be explained.

Cross-component changes require explicit scope or a blocker.

## Contract honesty

Reports must say:

```text
contract_read
contract_satisfied
contract_changes_requested
contract_change_rationale
affected_components
```

A contract-change request is preferred over an unsafe workaround.

## Checks honesty

Reports must distinguish:

```text
checks_run
checks_not_run
ci_status
workflow_urls
known_failures
```

Never claim a check passed unless it was executed or CI metadata was observed.

Connector-only work should explicitly say local shell checks were not run.

## Safety section

Reports must say whether the work added or exposed:

```text
secrets or sensitive data
unsafe public output
raw internal errors
provider calls
hard delete behavior
background jobs
```

Use safe summaries. Do not paste raw sensitive output.

## Next recommended agent

Use one of:

```text
orchestrator
implementation-worker
clean-code-reviewer
fixer-worker
architect
none
```

Recommended meanings:

- `clean-code-reviewer`: implementation self-accepted and should be reviewed.
- `fixer-worker`: CI or review failure needs a focused fix.
- `architect`: contract or dependency boundary needs design review.
- `orchestrator`: scheduling/fan-in/control decision is needed.
- `none`: no immediate follow-up is needed.

## Fan-in policy for active control files

Before merging component branches into an integration branch, Orchestrator must review active control files.

Recommended default:

```text
archive completed active prompt/report pairs into control/log
remove idle active prompt/report slots
keep control/state.md accurate
keep historical archived logs if they are safe and useful
```

Do not merge stale active prompts as if they were static docs.
