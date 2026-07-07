# W1-FIX-CMM-MERGEABILITY — Common mergeability fix

Component: common
Component path: crates/haze-sync-common
Branch: component/common
PR: #46

## Role

You are a Fixer Worker for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review.

## Context

The W1 common implementation and clean-code review are accepted pending CI, but PR #46 is not mergeable. Current observations:

- PR #46: mergeable=false
- merge_commit_sha: null
- component/common is ahead of main and behind current main
- no PR Component CI run exists for the current common head
- likely conflict/drift around `.github/workflows/component-ci.yml` and/or common component files

## Read

- crates/haze-sync-common/control/state.md
- crates/haze-sync-common/control/report.md
- crates/haze-sync-common/docs/component-contract.md
- crates/haze-sync-common/docs/implementation-plan.md
- current `main` versions of files that conflict or drift
- compare `main...component/common`
- PR #46 metadata

## Task

Restore mergeability for `component/common` while preserving accepted common component changes.

Preferred fix:

- align `.github/workflows/component-ci.yml` with current main policy if it is part of the conflict;
- keep common-owned source/docs/report changes intact;
- make only the minimum required conflict/sync edits.

If the GitHub connector cannot safely resolve the mergeability issue without local merge tooling, do not guess. Write a blocked report explaining the exact unresolved files and recommended local/Codex command sequence.

## Allowed files

- .github/workflows/component-ci.yml only if needed for mergeability
- crates/haze-sync-common/src/** only if needed to resolve drift/conflict
- crates/haze-sync-common/docs/** only if needed to resolve drift/conflict
- crates/haze-sync-common/control/report.md

Do not edit sibling components. Do not edit main. Do not merge.

## Checks

Use GitHub connector evidence only. Do not claim local checks passed. If CI is not available after fix, say so honestly.

## Report

Replace crates/haze-sync-common/control/report.md with a FIXER report.

Expected status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_TOOLING.
