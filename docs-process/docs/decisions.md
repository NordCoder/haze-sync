# Decisions: docs-process

## 2026-07-05 — Docs-process uses an isolated component root

Decision:

`docs-process` uses `docs-process/docs/**` as its component-local documentation root.

Rationale:

The repository may later use `docs/**` for product/system documentation. Keeping process docs under `docs-process/**` avoids ownership ambiguity between process conventions and system architecture docs.

Alternatives:

- Use `docs/**` directly for process component docs.
- Put process docs under `.github/docs/**`.
- Do not create a docs-process component root.

Consequences:

- Process docs are clearly separated from product/system docs.
- Future system docs can live under `docs/**` without conflicting with the process component.
- README may later link to this component root if desired.

Affected contracts:

- component contract;
- dependency map;
- implementation plan;
- future docs index work.

## 2026-07-05 — Docs-process documents workflow, it does not execute workflow

Decision:

Docs-process owns documentation of process conventions but does not create active prompts/reports, archive control files, open PRs, merge branches, run CI, or change branch protection.

Rationale:

Process documentation should clarify responsibilities without becoming an operational actor. Active control-slot mutation belongs to the assigned orchestration/worker lifecycle.

Alternatives:

- Let docs-process generate worker prompts.
- Let docs-process archive reports.
- Let docs-process decide merge readiness.

Consequences:

- Docs-process work remains behavior-neutral.
- Control lifecycle must be implemented by the assigned process roles.
- Any automation around prompts/reports requires a separate contract.

Affected contracts:

- control-slot guide;
- component contract;
- branch/process guide;
- report/status guide.

## 2026-07-05 — Local project-source prompts are not repository docs by default

Decision:

Project-source manifests, prompts, templates, local archives, and handoff notes are not automatically copied into the repository. They may inform tracked docs only when explicitly scoped and sanitized.

Rationale:

Local project-source context may include private operational instructions, prompt artifacts, or scratch material. Repository docs should contain stable, safe, public-enough process guidance.

Alternatives:

- Commit all local project-source files into the repo.
- Treat local prompt files as canonical tracked docs.
- Ignore process source context entirely.

Consequences:

- Docs-process must paraphrase and sanitize process rules when tracked docs are requested.
- Raw prompt/report/local archive files remain out of repo unless explicitly accepted.
- Public docs stay cleaner and safer.

Affected contracts:

- component contract;
- dependency map;
- docs hygiene phase;
- security/secrecy rules.

## 2026-07-05 — Component docs conventions do not override component ownership

Decision:

Docs-process may define conventions for component docs, but each component owns its own contract, plan, dependency map, decisions, and implementation log.

Rationale:

A centralized style guide is useful, but component ownership is the core mechanism that prevents cross-component drift and accidental scope expansion.

Alternatives:

- Let docs-process centrally rewrite all component contracts.
- Treat component docs as generated from one master file.
- Remove component-local docs in favor of root docs only.

Consequences:

- Bulk consistency edits require explicit scope.
- Contract contradictions should be reported, not silently overwritten.
- Component-local docs remain the first source for component-specific ownership.

Affected contracts:

- component docs guide;
- dependency map;
- component contracts;
- future drift audits.

## 2026-07-05 — Process docs must stay behavior-neutral unless explicitly scoped

Decision:

Docs-process changes are documentation-only by default. They must not change product code, CI workflow behavior, deployment automation, or runtime semantics unless a future prompt explicitly scopes cross-component work.

Rationale:

Process docs are often edited across the repo. Keeping them behavior-neutral reduces the risk of hidden product or operations changes in documentation branches.

Alternatives:

- Allow process doc phases to modify workflows and deployment scripts freely.
- Couple docs edits with product behavior changes.
- Treat process docs as executable policy.

Consequences:

- Product/runtime changes require their owning component branch and scope.
- CI/deployment changes require coordination with github-ci/deployment.
- Docs-process implementation reports should explicitly state behavior-neutral scope.

Affected contracts:

- implementation plan;
- dependency map;
- docs validation phase;
- system docs organization phase.

## 2026-07-05 — Public process docs are sanitized and repository-relative

Decision:

Process docs should use repository-relative paths and sanitized placeholders. They must not expose private paths, secrets, raw logs, provider payloads, or private scratch context.

Rationale:

Process docs are likely to be shared with workers/reviewers and may be read outside the immediate chat context. They must be safe and portable.

Alternatives:

- Copy raw local paths and logs into docs.
- Use real secrets in examples for clarity.
- Include private project scratch in tracked docs.

Consequences:

- Examples use placeholders.
- Failure examples must be summarized safely.
- Docs validation can later check for forbidden patterns.

Affected contracts:

- component contract;
- docs hygiene checks;
- report/status guide;
- branch/process guide.
