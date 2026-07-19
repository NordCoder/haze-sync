PROMPT_ID: component-smoke-github-only-20260719
CHAT_KEY: workflow-minimal-acceptance-component
ROLE: implementation-worker
COMPONENT: acceptance-smoke
REPOSITORY: NordCoder/haze-sync
CONTROL_BRANCH: agentic/workflow-minimal-acceptance/component-smoke
PRODUCT_BRANCH: none
INPUT_COMMIT: none

# Objective

Validate the minimal workflow GitHub control-slot semantics only. Do not modify product code and do not dispatch any browser chat.

# Authorized scope

Allowed:
- `.agentic/prompt.md`
- `.agentic/report.md`
- `.agentic/done.json`
- `.agentic/archive/**`

Forbidden:
- `main`
- `component/*`
- production or legacy control branches
- product code
- browser delivery
- automation changes

# Completion

A report without a matching done marker is non-terminal. Terminal publication must place the final matching report and done marker in one commit.
