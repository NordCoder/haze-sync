# W1-FIX-DEP-P5A-DOCUMENTATION-ALIGNMENT

Before starting, name this worker chat exactly:

`deployment — W1 DEP-P5A Documentation Alignment Fix`

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: fixer-worker
Phase: FIX-DEP-P5A-DOCUMENTATION-ALIGNMENT

Do not merge, change draft state, rewrite history, modify sibling branches, alter Compose behavior, or perform unrelated cleanup.

## Candidate

- DEP-P5A code-bearing SHA: `141878e549d9b05ffe0f4c1019bb31ee14b1bacd`;
- implementation report blob: `829fa86932d3b63ac8bfd578fd3f295f4218b135`;
- Component CI run `29404284889`, number `1959`, success.

The opt-in Worktree override, `.env.example` placeholders, base Compose comments and focused `deploy/docs/worktree-compose.md` are already implemented. Preserve them unless a factual contradiction requires the smallest correction.

## Required focused fix

Change only the documentation needed to remove historical contradictions:

1. `deploy/docs/host-directory-layout.md`
   - replace statements that Worktree bind mounts remain wholly future or forbidden;
   - state that the accepted opt-in override may bind an operator-supplied host path to `/var/lib/haze-sync/worktree`;
   - preserve the base Compose disabled/no-bind default;
   - preserve UID `10001`, non-world-writable, path-separation, backup and permission requirements;
   - state that the accepted override is read-only by default and write access remains an explicit operator gate.

2. `deploy/docs/migrations-backup-restore.md`
   - align writer lists and recovery-window text with the now-hostable Worktree runtime;
   - explicitly include Worktree data in the coordinated recovery set whenever it may contain authoritative or unreplicated content;
   - retain the architect-accepted ownership policy: human/operator executes manual SQLx migrations, Storage owns schema, Deployment owns sequencing;
   - preserve stop/quiesce before backup/migration/restore;
   - preserve no startup migration, no automatic rollback/down/reset/drop/destructive fallback;
   - link to the Worktree Compose runbook where useful.

3. Optionally add only narrow links from local/server Compose docs if required to prevent contradictory startup guidance.

## Protected behavior

Do not change:

- `deploy/docker-compose.worktree.yml` topology or interpolation;
- `deploy/docker-compose.yml` service behavior;
- `.env.example` keys/defaults;
- Server/API/CLI/Storage/Worktree product files;
- migration files, startup commands, entrypoints, healthchecks or workflows.

Do not add new architecture decisions. Do not broaden into a documentation rewrite.

## Validation

Confirm:

- all old “do not add a Worktree bind yet” statements are removed or correctly scoped to base Compose;
- documentation agrees on exact target `/var/lib/haze-sync/worktree`;
- base disabled/no-bind and override opt-in/read-only defaults remain clear;
- migration ownership and recovery-window rules remain exact;
- no real host path, secret, credential, dump or archive is introduced;
- final exact-SHA Component CI is green.

Create a real documentation-bearing commit without CI skip.

Write `deploy/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-DEP-P5A-DOCUMENTATION-ALIGNMENT`;
- `chat_name: deployment — W1 DEP-P5A Documentation Alignment Fix`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record changed paths, contradictions removed, policy preservation, final SHA and exact CI. Do not claim CLEAN_ACCEPT; a focused functional/security review follows.
