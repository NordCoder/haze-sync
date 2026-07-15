# W1-FIX-OBS-FAN-IN-P1-REVIEW

Before starting, name this worker chat exactly:

`obsidian-plugin — W1 FIX-OBS-FAN-IN-P1 Review`

Repository: `NordCoder/haze-sync`
Component: obsidian-plugin
Path: `apps/haze-obsidian-plugin`
Branch/ref: `component/obsidian-plugin`
PR: #51
Role: fixer-worker
Phase: `FIX-OBS-FAN-IN-P1-REVIEW`

This is a focused fixer for the existing `OBS-FAN-IN-P1-SERVER-COMPAT-E2E` phase. Do not begin another Obsidian product or release phase.

## Review target and findings

- reviewed code-bearing SHA: `2f03dc49e7fdff8cfd0e4685c07ffc0ba75602ce`;
- clean-review report blob: `aa4fe184ec398168d805412a9d709e4dcca464b7`;
- authoritative green CI before review findings: run `29434935575`, number `2000`.

The review found two component-local defects:

1. Absolute-path redaction is incomplete for valid common forms, including paths with spaces, Windows forward-slash paths, UNC paths, `/mnt`, `/Volumes`, and Android-style `/storage` roots.
2. The deterministic download fixture returns bytes whose advertised SHA-256 does not match, and the test does not exercise the production remote-materializer hash verifier.

## Required fix

1. Harden the API-client public-error sanitizer so complete recognizable absolute local paths are removed without exposing suffixes for supported Windows, UNC, Linux, macOS and Android-style forms, including whitespace-containing paths.
2. Add focused positive and negative redaction tests. Preserve safe non-path public text where practical; do not simply erase every slash-containing string.
3. Correct the successful fake download hash/body pairing.
4. Exercise the production hash-verification boundary directly through `materializeRemoteChange` or the actual production-equivalent entrypoint:
   - matching metadata/body succeeds;
   - mismatched metadata/body fails safely before vault mutation;
   - no cursor/base revision is advanced on mismatch.
5. Preserve deterministic fake transport, loopback-only optional smoke behavior, synthetic-only fixtures and existing endpoint/header semantics.

## Scope

Allowed only where necessary:

- `apps/haze-obsidian-plugin/src/api-client/errors.ts`;
- production remote materializer or its test-facing adapter only if required to invoke the existing verifier without redesign;
- `apps/haze-obsidian-plugin/tests/**`;
- minimal test/docs alignment;
- control report.

Forbidden:

- API, Server or sibling component changes;
- route/DTO invention;
- workflow changes;
- external-network-required CI;
- real vault data or credentials;
- weakening existing tests;
- release packaging or marketplace work;
- unrelated production refactors;
- merge, rebase, force-push or PR draft-state change.

## Validation

Create code/test changes without CI skip. Obtain a new full exact-SHA Component CI run with npm install, plugin tests, typecheck, build, Node diagnostics finalization and Rust workspace green. Optional loopback smoke may remain honestly skipped without local synthetic configuration. PR #51 remains open, draft and unmerged.

## Report

Write only `apps/haze-obsidian-plugin/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-OBS-FAN-IN-P1-REVIEW`;
- `chat_name: obsidian-plugin — W1 FIX-OBS-FAN-IN-P1 Review`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record exact changed paths, redaction cases covered, production hash-verifier scenarios, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT; a repeat clean integration review follows.
