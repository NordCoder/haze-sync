# Decisions: github-ci

## 2026-07-05 — CI validates repository quality, not production readiness

Decision:

GitHub CI validates source quality and repository scaffolds. It does not prove that Haze Sync is production-deployed, connected to real providers, or safe for live bidirectional sync.

Rationale:

Current checks cover Rust workspace quality, plugin typecheck/build, and compose config syntax. Those are necessary but not sufficient for production readiness.

Alternatives:

- Treat passing CI as production readiness.
- Add live provider/deploy tests immediately.
- Remove deployment scaffold validation entirely.

Consequences:

- CI documentation must state what checks prove and do not prove.
- Production readiness remains Deployment/Server/GDrive/Worktree runbook responsibility.
- Future E2E jobs should be explicit and staged.

Affected contracts:

- component contract;
- implementation plan;
- Deployment validation;
- README/CI expectations.

## 2026-07-05 — Default CI is secret-free

Decision:

Default CI jobs must not require production secrets, Google OAuth credentials, bearer tokens, production database URLs, TLS private keys, real vault data, or remote host access.

Rationale:

Default CI runs broadly on pull requests and pushes. Secret-dependent checks increase leak risk and reduce contributor reproducibility.

Alternatives:

- Require production credentials for E2E checks.
- Store deployment secrets for default CI early.
- Skip all integration validation to avoid secrets.

Consequences:

- Provider tests should be fake-provider by default.
- Deployment validation should remain syntax/local checks unless scoped.
- Real-provider or deployment workflows need separate manual/secured contracts.

Affected contracts:

- workflow files;
- GDrive adapter tests;
- deployment validation;
- future E2E jobs.

## 2026-07-05 — Workflow permissions stay least-privilege

Decision:

Workflow permissions should remain minimal for normal validation jobs. Current `ci.yml` uses read-only contents permission.

Rationale:

Most checks only need source checkout. Broad write permissions would create unnecessary risk, especially before release/deploy automation exists.

Alternatives:

- Grant write permissions globally for convenience.
- Add package/deploy permissions before release policy exists.
- Use repository secrets and broad tokens for all jobs.

Consequences:

- Future release/deploy workflows need separate permissions and manual gates.
- Validation workflows remain low-risk.
- Permission changes require documentation and review.

Affected contracts:

- workflow files;
- release/deploy planning;
- security policy.

## 2026-07-05 — Compose validation is syntax/local scaffold validation only

Decision:

`docker compose -f deploy/docker-compose.yml config` validates the local compose file syntax. It does not start services and does not prove production deployment readiness.

Rationale:

Current compose contains local PostgreSQL only. Server/GDrive services, secrets, reverse proxy, migrations, backup/restore, and sync bootstrap are deferred.

Alternatives:

- Treat compose config validation as production deployment check.
- Start full sync stack in CI before services are ready.
- Drop compose validation until production deployment exists.

Consequences:

- README and CI docs should use precise wording.
- Deployment component owns production runbooks later.
- CI can add richer local integration only after contracts stabilize.

Affected contracts:

- CI workflow;
- Deployment docs;
- README check descriptions;
- future integration jobs.

## 2026-07-05 — Workflow trigger changes affect process policy

Decision:

Changes to workflow triggers, job names, and required-check coverage can affect branch/process behavior and merge readiness. They require documentation and process coordination.

Rationale:

GitHub required checks are external repository settings. Renaming/removing jobs or changing branch triggers can silently break expected review/merge gates.

Alternatives:

- Change job names freely.
- Disable PR validation during component work.
- Add branch triggers without documenting them.

Consequences:

- Trigger/job changes belong in scoped CI phases.
- Implementation reports should call out required-check impacts.
- Process/docs may need updates alongside CI changes.

Affected contracts:

- workflow files;
- process docs;
- orchestrator/merger expectations;
- required status checks.

## 2026-07-05 — Release/deploy automation is deferred

Decision:

No release publishing, production deployment, remote host mutation, or secret-using deploy workflow is part of current CI. Such workflows require future explicit contracts.

Rationale:

The repository is still building product foundations and component docs. Production automation would introduce operational and security risks before rollout/runbook readiness.

Alternatives:

- Auto-deploy from `main`.
- Publish binaries/plugins on every tag immediately.
- Add SSH-based deploy workflows now.

Consequences:

- CI remains validation-only for now.
- Deployment component owns runbooks and future service topology first.
- Release/package workflows can be added later with manual gates and artifact policy.

Affected contracts:

- CI workflow plan;
- Deployment plan;
- release/package policy;
- secret policy.
