# Implementation Log: cli

## Entries

### 2026-07-05 — CLI-P1 component contract and planning normalization

Agent:
Architect

Branch:
component/cli

Prompt:
User requested continuing component documentation and implementation-plan writing after GDrive adapter.

Report:
Conversation summary; no component control report was written because this is an Architect documentation/planning pass, not an implementation-worker execution.

Commit(s):
See branch history after this documentation pass.

Summary:
Replaced scaffold-level CLI docs with a real component contract, dependency map, implementation plan, decisions, and this planning baseline log entry. The pass documented CLI as an operator-facing command surface, not Core policy owner, Server runtime owner, Storage persistence owner, provider adapter, Worktree runtime, or Obsidian plugin. Current behavior remains read-only placeholders and offline doctor summary only. Future phases cover parser/output hardening, config/secret handling, live status/adapters, live doctor, bootstrap/sync workflows, admin mutation commands, and output/packaging readiness.

Status:
ARCHITECT_ACCEPT_PENDING_REVIEW

Follow-ups:
Execute CLI-P2 through CLI-P8 through normal implementation -> clean-code -> CI -> fixer lifecycle when scheduled. Keep CLI output secret-safe, live behavior Server/API-backed, and admin/destructive commands explicitly scoped with confirmation and dry-run behavior where practical.

---

### 2026-07-06 — W1/CLI-P2 parser and output contract hardening

Agent:
Implementation Worker

Branch:
component/cli

Prompt:
`crates/haze-sync-cli/control/prompt.md`

Report:
`crates/haze-sync-cli/control/report.md`

Commit(s):
See branch history for this implementation pass.

Summary:
Unified current CLI parsing under the top-level command model, routed `doctor` through that model, added explicit stdout/stderr/exit-code output helpers, hardened parse errors so they do not echo raw argument values, and documented the current read-only command categories and decision to keep the dependency-free manual parser for CLI-P2.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run clean-code review and CI/shell checks. Future phases still own config/secret-source handling, live Server/API calls, live doctor behavior, bootstrap/admin commands, and stable machine-readable output.

---

### 2026-07-09 — W1/CLI-P3 config and secret-source foundation

Agent:
Implementation Worker

Branch:
component/cli

Prompt:
`crates/haze-sync-cli/control/prompt.md`

Report:
`crates/haze-sync-cli/control/report.md`

Commit(s):
See branch history for this implementation pass.

Summary:
Added a CLI-local config module with explicit source precedence for server URL, profile, output format, and token source. The new token-source model supports safe descriptors for env var, config file path, stdin, OS-secret reference, or none, while rejecting inline token values. Config debug and report summaries redact server URLs, file paths, OS-secret references, and token-source values. The phase added tests for precedence, accepted token-source descriptors, inline token rejection, redacted debug/summary output, and invalid config errors that do not echo raw values. No env/file/stdin/keychain/server/provider IO was added.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run clean-code review and CI/shell checks. Future phases still own actual config loading, HTTP client wiring, live Server/API commands, live doctor behavior, token creation/rotation, and OS-secret integration if accepted.

---

Use this format for future entries:

~~~text
### YYYY-MM-DD — <wave>/<phase>

Agent:
Branch:
Prompt:
Report:
Commit(s):
Summary:
Status:
Follow-ups:
~~~
