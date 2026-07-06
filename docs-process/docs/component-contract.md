# Component Contract: docs-process

## Responsibility

`docs-process` owns repository process documentation and documentation-system conventions for Haze Sync.

The component is responsible for future guidance around:

- component documentation structure;
- component planning document conventions;
- control-slot conventions for prompts/reports/logs;
- report and status vocabulary alignment;
- branch/process documentation;
- system documentation index conventions;
- operator/developer documentation organization;
- documentation quality gates that do not change product behavior.

This component is a process/documentation component. It does not own runtime sync behavior, product API semantics, code implementation, CI workflow behavior, or deployment automation.

Current state before this pass:

- branch `component/docs-process` had no dedicated `docs-process/docs/**` files;
- process rules exist primarily in project-source context and component-local docs across other components;
- root README contains product and CI overview, but not a dedicated docs-process contract.

This pass creates the component-local documentation baseline.

## Public interfaces

Current public files introduced by this component:

```text
docs-process/docs/component-contract.md
docs-process/docs/implementation-plan.md
docs-process/docs/dependency-map.md
docs-process/docs/decisions.md
docs-process/docs/implementation-log.md
```

Future public documentation surfaces may include:

```text
docs-process/docs/component-docs-guide.md
docs-process/docs/control-slot-guide.md
docs-process/docs/reporting-guide.md
docs-process/docs/branch-lifecycle.md
docs-process/docs/status-vocabulary.md
docs-process/docs/system-docs-style-guide.md
docs/README.md
docs/system-architecture.md
docs/component-boundaries.md
```

Product/system docs under `docs/**` may be owned or coordinated by this component only when a prompt explicitly scopes system documentation work.

## Input contracts

Inputs include:

- current component docs;
- root README and repository layout;
- implementation manifest/process rules from project sources when explicitly referenced by user prompts;
- report template/status vocabulary from project sources when explicitly referenced;
- component control folders and archived prompt/report logs;
- branch names and component lifecycle conventions;
- implementation reports from workers/reviewers/fixers;
- CI/deployment docs where process wording must align.

Required input rules:

- do not treat untracked local project-source prompts as repository product docs unless explicitly scoped;
- preserve process semantics when converting local instructions into tracked docs;
- distinguish process docs from product architecture docs;
- avoid embedding live prompts, reports, local scratch, secrets, logs, dumps, or private context into public docs;
- keep documentation changes behavior-neutral unless explicitly scoped otherwise.

## Output contracts

Outputs include tracked Markdown process/docs files.

Required output rules:

- docs must be dry, explicit, and component-oriented;
- docs must state ownership and non-goals;
- docs must distinguish current implementation from target/future work;
- docs must not claim CI, product, or deployment readiness not proven by the repo;
- docs must not include secrets, local absolute user paths, tokens, token hashes, provider payloads, raw logs, or private project scratch;
- docs must not create active worker prompts/reports unless the current task explicitly says so.

## Error contracts

Docs-process output must not expose:

- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values;
- database URLs;
- production `.env` contents;
- provider payloads;
- local absolute user paths;
- stack traces;
- private chat/scratch content;
- raw CI logs containing sensitive data.

When documenting failures or examples, use sanitized placeholders and repository-relative paths.

## Persistence/runtime ownership

Docs-process owns tracked documentation and process conventions only.

Docs-process may own:

- repository process docs;
- documentation templates and style conventions;
- component docs structure guidance;
- control-slot lifecycle docs;
- status/report vocabulary docs;
- process branch documentation;
- system documentation index/organization when scoped.

Docs-process does not own:

- Core/API/Storage/Server runtime behavior;
- adapter/plugin/CLI/Deployment/CI implementation;
- GitHub repository settings;
- branch protection settings;
- PR creation/merge decisions;
- product architecture changes unless explicitly scoped as docs-only recording of accepted decisions;
- local project-source files outside the repository.

## Security and secrecy rules

- Do not commit prompts, reports, local scratch, local handoff archives, or private project-source files unless explicitly accepted.
- Do not copy secret-bearing logs or real `.env` values into docs.
- Do not expose private chain-of-thought, private credentials, or raw provider payloads.
- Keep examples sanitized and placeholder-based.
- Keep active control slots owned by the Orchestrator/Worker protocol; docs-process must not archive or mutate active prompts/reports unless explicitly scoped.
- Do not change product behavior while editing process docs.

## Non-goals

Docs-process must not implement:

- product code;
- CI workflow changes unless coordinated with github-ci;
- deployment automation;
- server route behavior;
- API DTO changes;
- storage migrations;
- Core sync policy;
- adapter/provider behavior;
- Obsidian plugin behavior;
- CLI runtime behavior;
- branch protection or GitHub settings changes;
- active prompt/report generation for implementation workers unless explicitly assigned.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Process docs describe the workflow; they do not execute it.
- Component docs are local to component roots and must preserve ownership boundaries.
- Current state and target state must be separated.
- Control prompt/report slots must not be pre-created as documentation artifacts.
- Public docs must be safe to read outside the project context.
- Product behavior remains unchanged by docs-process work.

## Test obligations

Docs-process checks should eventually cover:

- Markdown link validity where tooling exists;
- no forbidden prompt/report/archive files added accidentally;
- no secret-like tracked docs examples;
- component docs structure consistency;
- README/docs index consistency;
- no broken references to component paths;
- alignment between process docs and active repository branch policy.

Manual review obligations for docs-process changes:

- verify behavior-neutral scope;
- verify process wording does not contradict component contracts;
- verify examples are sanitized;
- verify current-state claims match repository state.

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- changing component lifecycle rules;
- changing branch policy;
- changing active prompt/report slot ownership;
- committing local project-source prompt/report files;
- changing product architecture or runtime behavior;
- editing CI workflow policy without github-ci coordination;
- editing deployment automation without deployment coordination;
- exposing private/sensitive project context in public docs.
