# CI Diagnostics Artifacts

Status: active harness policy
Scope: PR Component CI
Repository: `NordCoder/haze-sync`

---

## Purpose

This document defines the diagnostics artifact interface used by Fixer Workers to read CI failure output through the GitHub connector.

The normal loop is:

```text
component branch has an open PR to main
worker pushes to the component branch
pull_request synchronize CI runs against the PR merge branch
CI failure uploads one component-scoped diagnostics artifact
Orchestrator observes red CI status and writes a component fixer prompt
Fixer Worker downloads and reads the artifact
```

The Orchestrator does not normally download or inspect diagnostics artifacts.

---

## Trigger policy

Diagnostics artifacts are produced by `Component CI` on pull requests targeting `main`.

The workflow relies on the GitHub `pull_request` event. For open PRs, a push to the PR head branch triggers the `synchronize` activity and reruns PR CI.

`Component CI` should not run on plain component branch pushes in this harness policy. This avoids duplicate CI runs and duplicate diagnostics artifacts.

Expected trigger shape:

```yaml
on:
  pull_request:
    branches:
      - main
  workflow_dispatch:
```

---

## Responsibilities

### GitHub Actions runner

The runner:

- runs the normal CI checks;
- streams check output to the Actions UI;
- records logs only for failed checks;
- writes `ci-diagnostics/summary.md` and `ci-diagnostics/manifest.json`;
- uploads a diagnostics artifact only when the job fails.

### Orchestrator

The Orchestrator:

- checks PR CI status through the GitHub connector;
- if CI is green, proceeds to the next lifecycle prompt;
- if CI is red, writes a Fixer Worker prompt for the component;
- tells the worker to read the component diagnostics artifact.

The Orchestrator should not download artifacts during normal flow.

### Fixer Worker

The Fixer Worker:

- reads its active control prompt;
- uses the GitHub connector to find/download the diagnostics artifact for its component PR CI run;
- reads `summary.md`, `manifest.json`, and logs listed in `failed_checks`;
- fixes the minimum cause inside component scope;
- reports honestly if logs are missing or the fix requires another component/contract.

---

## Component ownership

The component is derived from the PR head branch:

```text
component/core             -> core
component/storage          -> storage
component/api              -> api
component/server           -> server
component/common           -> common
component/cli              -> cli
component/worktree         -> worktree
component/gdrive-adapter   -> gdrive-adapter
component/obsidian-plugin  -> obsidian-plugin
component/deployment       -> deployment
component/github-ci        -> github-ci
component/docs-process     -> docs-process
```

If the PR head branch is not `component/<name>`, the workflow uses `unknown` as the component name. A Fixer Worker should not treat `unknown` diagnostics as component-owned without explicit Orchestrator direction.

---

## Artifact naming

Diagnostics artifact names must include component, workflow slug, run id, and attempt:

```text
ci-diag__component-<component>__wf-<workflow-slug>__run-<run-id>__attempt-<attempt>
```

Examples:

```text
ci-diag__component-core__wf-component-ci__run-987654321__attempt-1
ci-diag__component-api__wf-component-ci__run-987654322__attempt-1
ci-diag__component-obsidian-plugin__wf-component-ci__run-987654323__attempt-1
```

Dates are not used as primary identifiers. GitHub run id and attempt map directly to connector workflow run/artifact APIs.

---

## Artifact retention and upload policy

Diagnostics artifacts are transient.

Required upload policy:

```yaml
retention-days: 1
compression-level: 9
include-hidden-files: false
if-no-files-found: ignore
```

Rules:

- upload only on failure;
- do not upload artifacts for green jobs;
- upload at most one diagnostics artifact per failed job;
- store only failed check logs;
- do not store successful check logs;
- do not store raw full workflow archives.

---

## Artifact structure

Required structure:

```text
ci-diagnostics/
  manifest.json
  summary.md
  failures/
    <check-name>.txt
  logs/
    <check-name>.log
```

Example:

```text
ci-diagnostics/
  manifest.json
  summary.md
  failures/
    rust-fmt.txt
    cargo-test.txt
  logs/
    rust-fmt.log
    cargo-test.log
```

Logs are full failed-check logs in v1. There is no tail/cap truncation in v1.

---

## Manifest schema

Example:

```json
{
  "schema": "haze-ci-diagnostics-v1",
  "component": "core",
  "component_branch": "component/core",
  "workflow": "Component CI",
  "workflow_slug": "component-ci",
  "run_id": "987654321",
  "run_attempt": "1",
  "run_number": "42",
  "event": "pull_request",
  "head_sha": "d53af004e36bc793b040170bad1196fb0501b77b",
  "created_at_utc": "2026-07-09T18:20:00Z",
  "retention_days": 1,
  "failed_checks": [
    {
      "check": "rust-fmt",
      "exit_code": 1,
      "log": "logs/rust-fmt.log",
      "failure_marker": "failures/rust-fmt.txt"
    }
  ]
}
```

Required fields:

```text
schema
component
component_branch
workflow
workflow_slug
run_id
run_attempt
head_sha
retention_days
failed_checks
```

---

## Check execution mode

Component CI uses aggregate diagnostics mode:

- all relevant checks run;
- failed checks are recorded;
- later checks still run after earlier failures;
- finalization fails the job if any failure marker exists;
- the uploaded artifact contains logs only for failed checks.

This prevents a fixer loop where the first run reveals only formatting, the second run reveals tests, and a later run reveals clippy.

---

## Forbidden artifact contents

Diagnostics artifacts must not contain:

```text
target/
node_modules/
dist/
coverage/
.env
.env.*
database dumps
object-store data
vault contents
OAuth token files
provider payloads
production DB URLs
SSH keys
TLS private keys
raw environment dumps
local notes
generated archives unrelated to CI diagnostics
```

Allowed contents:

```text
summary.md
manifest.json
failures/*.txt
logs/*.log for failed checks only
```

---

## Missing artifact policy

If CI is red but the diagnostics artifact is missing or expired, the Fixer Worker must not guess CI details.

Expected report status:

```text
FIX_BLOCKED_BY_LOGS
```

Raw GitHub job logs may be used only as a fallback when explicitly directed by the active prompt.
