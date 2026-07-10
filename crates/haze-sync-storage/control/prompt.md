# W1-STOR-P9C-BLOCKED-BY-SERVER-CONTRACT — Production feature-isolation gate

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted Storage state

STOR-P9 implementation, artifact-based CI correction, and Storage-local clean-code fixes are complete. Final Storage source/docs CI is green.

- code_bearing_sha: aa59064d641f4850f7c70fa615e638b52613dd95
- workflow: Component CI
- workflow_run_id: 29124956486
- run_number: 1580
- conclusion: success

## Cross-component blocker

`crates/haze-sync-server/Cargo.toml` currently enables `haze-sync-storage` feature `test-support` in Server's normal dependency declaration. Cargo feature unification therefore makes test-only Storage APIs part of normal Server production compilation, violating Storage's accepted production-isolation contract.

The required correction belongs to the Server component:

- keep normal Server dependency on `haze-sync-storage` without `test-support`;
- enable `test-support` only for Server dev/test builds or a dedicated test harness;
- preserve Server tests and normal production compilation;
- rerun full Component CI.

## Unblock condition

The Server-owned dependency correction is accepted with green CI and proves normal production builds no longer enable Storage `test-support`. Until then, do not launch a Storage worker from this hold notice.
