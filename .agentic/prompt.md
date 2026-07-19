WORKFLOW: workflow-minimal
CHAT_KEY: orchestrator
PROMPT_ID: orch-plan-20260719-01
ROLE: orchestrator
COMPONENT: none
REPOSITORY: NordCoder/haze-sync
CONTROL_BRANCH: agentic/workflow-minimal-acceptance/orchestrator
PRODUCT_BRANCH: none
INPUT_COMMIT: none
WORK_ITEM: none
PLAN_SOURCE: haze-sync-v1-completion-plan.md
PLAN_WAVE: none
PLAN_SECTIONS:
- full plan

# Plan authority

Current completion plan:
haze-sync-v1-completion-plan.md

Baseline plans:
- haze-sync-development-wave-plan.md

Architecture sources:
- haze-sync-docs-v2

The current completion plan is authoritative for remaining work, work-item identity, dependency ordering, waves, fan-in and completion boundaries. Baseline plans provide historical and architectural context only. Verify all plan assumptions against current GitHub state before creating new prompts. Do not silently redesign the plan. When current evidence invalidates a plan item, report the exact discrepancy and prepare only safe bounded work.

Requested plan wave: none
Specific work items:
- none

If specific work items are supplied, do not create tasks outside that list unless this prompt explicitly permits recalculating the batch.

Additional evidence:
This is an operator-directed planning turn. The Dispatcher is only delivering this Orchestrator trigger; it must not choose worker assignments.

# Objective

Read the current completion plan and verified repository evidence, then select the next bounded workflow-minimal assignments. Do not perform product work. Prepare only the exact worker prompts and terminal evidence required by the plan.

# Context

No additional context.



# Authorized scope

Allowed:
- the branches and paths explicitly named above

Forbidden:
- unrelated branches and paths; force-push; additional assignments; unrelated changes.

# Completion

Write `.agentic/report.md`. Repeat WORK_ITEM, PLAN_SOURCE and relevant PLAN_SECTIONS in the report when applicable. When terminal, create `.agentic/done.json` with the same PROMPT_ID and publish report plus done marker in one control-branch commit.
