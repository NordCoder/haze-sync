# Control Folder

This folder is the active-slot interface between Orchestrator and agents.

## Files

- `state.md` — current control-slot status.
- `prompt.md` — active prompt, created by Orchestrator only.
- `report.md` — active/latest report, written by the assigned agent.
- `log/` — archived prompts and reports.

## Rules

- Orchestrator owns prompt creation and archiving.
- Workers read `prompt.md`.
- Workers write `report.md`.
- Workers must not archive files unless explicitly instructed.
- Do not store secrets, token hashes, raw CI dumps with credentials, generated artifacts, or local notes here.
