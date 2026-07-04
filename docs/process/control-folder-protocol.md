# Control Folder Protocol

## Active-slot invariant

At any time, component `control/` may contain only one active prompt and one active/latest report:

~~~text
control/prompt.md
control/report.md
control/state.md
control/log/
~~~

`prompt.md` and `report.md` are optional active files. They are not created in idle scaffolds.

## Ownership

Orchestrator owns:

- creating `prompt.md`;
- archiving old `prompt.md` and `report.md`;
- updating `state.md`;
- choosing the next agent.

Workers own:

- reading `prompt.md`;
- writing `report.md`.

## Archive naming

~~~text
YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-prompt.md
YYYYMMDD-HHMMSSZ-<wave>-<phase>-<agent-role>-report.md
~~~

## State values

Common states:

- `IDLE`
- `PROMPT_READY`
- `WORKER_RUNNING`
- `REPORT_READY`
- `ORCHESTRATOR_REVIEW`
- `CI_PENDING`
- `CI_RED`
- `CI_GREEN`
- `BLOCKED`
