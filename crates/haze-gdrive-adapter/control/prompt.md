# W1-GDA-GDA-P3-ACCEPTED-DEPENDENCY-HOLD

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: orchestrator-hold
Phase: GDA-GDA-P3-ACCEPTED-DEPENDENCY-HOLD

This is a hold notice, not an executable worker prompt.

Accepted OAuth/credential/auth candidate:
- code-bearing SHA: `7f00a60641ca157907d0e75e4ab1bb47c05f03c9`;
- clean-review report blob: `27a465aabd66975f2c519516d8086292639d5cd0`;
- Component CI run: `29486777334`, run number `2032`, success.

Accepted surface:
- strict bounded read-only credential-file contract;
- fakeable token and Google provider client boundaries;
- memory-only access-token refresh;
- deterministic caller-provided time boundary and minimum-lifetime validation;
- safe revoked/scope/refresh/provider categories;
- observation-only startup preflight;
- credential/token/request/path/provider-body redaction;
- fake-only tests without real credentials or live-network CI.

Next dependency gate:
- `GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT` requires accepted Server GDrive state routes and transaction semantics;
- Server is currently in `SRV-GDA-P1-CLEAN-FUNCTIONAL-REVIEW`;
- do not start P2 or the long-running runtime until Server receives CLEAN_ACCEPT and Orchestrator supplies the exact accepted Server SHA/report evidence.

Do not implement, merge, change draft state, rewrite history, modify sibling branches, workflows or product files. Wait for an explicit Orchestrator `PROMPT_READY` assignment.
