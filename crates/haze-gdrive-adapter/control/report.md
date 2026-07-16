REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P3 OAuth Expiry

COMPONENT:
name: gdrive-adapter
branch: component/gdrive-adapter
pr: 50

WAVE:
id: W1
phase_id: FIX-GDA-GDA-P3-REVIEW

REVIEW_TARGET:
code_bearing_sha: cf86aba890df6dcfb95f1d285677ceacf96a7772
clean_review_report_blob: ec281ebc5f3f5343887d339d7f676977587ab738
reviewed_ci_run_id: 29456659157
reviewed_ci_run_number: 2026

SUMMARY:
Closed the OAuth access-token expiry lifecycle defect. AccessToken construction no longer reads a hidden wall clock. GoogleAuthClient::access_token(now) is now the authoritative deterministic time boundary for both cached-token and post-refresh validation. A refreshed token below MIN_TOKEN_LIFETIME for the caller-provided now is rejected as retryable RefreshUnavailable and is not cached. Added focused fixed-time tests for expired cached refresh, usable refreshed caching/reuse, too-short refreshed rejection/non-caching, and unchanged valid cached-token behavior.

CHANGED_PATHS:
- crates/haze-gdrive-adapter/src/auth.rs
- crates/haze-gdrive-adapter/control/report.md

DETERMINISTIC_TIME_MODEL:
- AccessToken::new stores provider token value and absolute expires_at only.
- AccessToken construction performs no SystemTime::now() read.
- AccessToken::is_usable_at(now) remains the single minimum-lifetime predicate.
- GoogleAuthClient::access_token(now) applies that predicate to cached tokens.
- Every refreshed token is validated with the same caller-provided now before assignment.
- Tokens with remaining lifetime below 30 seconds are rejected as RefreshUnavailable.
- An unusable refreshed token clears the cache and is never returned.

LIFECYCLE_TESTS:
- expired_cached_token_causes_refresh
- usable_refreshed_token_is_cached_and_reused
- too_short_refreshed_token_is_rejected_and_not_cached
- valid_cached_token_is_returned_without_refresh
- all lifecycle tests use fixed SystemTime values and no wall-clock reads

PRESERVED:
- read-only versioned credential-file contract
- memory-only access-token lifecycle
- fakeable TokenEndpoint and GoogleProviderClient abstractions
- safe auth/scope/provider categories and retry classification
- observation-only preflight
- credential/token/request/path/provider-body redaction
- synthetic fake-only tests
- no credential writes or live network
- no Server/API, Storage, scheduler, Deployment, sibling, workflow, dependency, merge, rebase, or draft-state changes

INTERIM_CI_DIAGNOSTICS:
interim_code_sha: c084eb4a7711e1e1a9677d5dfbd9747c3b5d7e70
run_id: 29486545078
run_number: 2031
artifact_id: 8370643768
artifact_files_read:
- summary.md
- manifest.json
- failures/cargo-clippy.txt
- failures/cargo-test.txt
- failures/rust-fmt.txt
- logs/cargo-clippy.log
- logs/cargo-test.log
- logs/rust-fmt.log
artifact_causes:
- one test retained a borrowed token reference while reading the endpoint counter
- rustfmt required three layout-only corrections
minimum_followup_commit: 7f00a60641ca157907d0e75e4ab1bb47c05f03c9

FINAL_CODE_BEARING_SHA:
7f00a60641ca157907d0e75e4ab1bb47c05f03c9

CI:
workflow: Component CI
run_id: 29486777334
run_number: 2032
exact_sha: 7f00a60641ca157907d0e75e4ab1bb47c05f03c9
conclusion: success
checks:
- cargo fmt: success
- cargo check: success
- cargo test: success
- cargo clippy: success
- Finalize CI diagnostics: success
- Upload CI diagnostics: skipped because no failures

PR_STATE:
open: yes
draft: yes
merged: no
mergeable: yes

CI_SKIP:
used: yes
reason: final report-only commit after the exact final code-bearing SHA passed complete CI

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
clean-code-reviewer

FINAL_VERDICT:
FIX_COMPLETE. The hidden wall-clock dependency is removed, refreshed tokens are validated against the caller-provided time before caching or return, too-short tokens fail closed without entering the cache, deterministic lifecycle tests cover every required path, and exact SHA 7f00a60641ca157907d0e75e4ab1bb47c05f03c9 passed full Component CI run 29486777334. Repeat focused OAuth/security clean review is required; no CLEAN_ACCEPT claim is made.

PUSHED:
yes
