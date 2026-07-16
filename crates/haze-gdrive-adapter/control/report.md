REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
chat_name: gdrive-adapter — W1 GDA-GDA-P3 OAuth Security Review

COMPONENT:
name: gdrive-adapter
branch: component/gdrive-adapter
pr: 50

WAVE:
id: W1
phase_id: GDA-GDA-P3-CLEAN-REVIEW

REVIEW_TARGET:
code_bearing_sha: cf86aba890df6dcfb95f1d285677ceacf96a7772
implementation_report_blob: 5f0aea2859eba8575f3c8e302b72a09ca7aa24fc
fixer_report_blob: e33d265a28c2f56fff60d5e528c4f7dfd6a0e2a8

SUMMARY:
The credential parser, read-only file lifecycle, redaction, safe categories, fakeable boundaries and observation-only preflight are present. However, the token refresh lifecycle does not satisfy the required bounded deterministic expiry contract. AccessToken::new validates expiry against an internal SystemTime::now(), while GoogleAuthClient::access_token accepts an explicit now. After a refresh, the returned token is stored and returned without rechecking usability against that explicit now. A token valid relative to wall-clock construction time but already unusable relative to the caller-provided now can therefore be returned. The existing expired_token_refreshes_in_memory test does not create or replace an expired cached token; it only exercises an initially empty cache and uses wall-clock time, so it does not close this defect.

FINDINGS:
- severity: high
  title: Refreshed access token can be returned below the minimum lifetime for the caller's time boundary
  path: crates/haze-gdrive-adapter/src/auth.rs
  evidence:
  - AccessToken::new computes remaining lifetime against SystemTime::now() rather than an explicit deterministic reference time.
  - GoogleAuthClient::access_token decides refresh using its now argument, but after endpoint.refresh it returns the refreshed token without calling is_usable_at(now).
  - The endpoint can therefore return a token accepted at construction time but invalid or below MIN_TOKEN_LIFETIME relative to the access_token(now) call.
  - expired_token_refreshes_in_memory starts with access_token=None, creates a five-minute token using SystemTime::now(), and calls access_token(SystemTime::now()); it does not test an expired cached token or a too-short refreshed token.
  impact:
  - Authorization code may receive an already expired or imminently expiring token despite the minimum-lifetime guarantee.
  - Time-sensitive behavior is not deterministic under fake tests because construction reads the wall clock internally.
  required_fix:
  - Make token construction/validation use an explicit reference time or represent provider expiry data without hidden wall-clock validation.
  - After refresh, validate the returned token with the same caller-provided now before storing or returning it; fail safely as RefreshUnavailable when below the minimum lifetime.
  - Add deterministic tests for an expired cached token refreshing, a refreshed token acceptable for the caller time, and a refreshed token below the minimum lifetime being rejected.

REVIEW_RESULTS:
versioned_bounded_parser_strict_fail_closed: yes
credential_file_read_only: yes
refreshed_tokens_memory_only: yes
secret_path_and_output_redaction: yes for reviewed surfaces
expiry_handling_bounded_and_deterministic: no
safe_auth_scope_provider_categories: yes for represented errors
retry_classification: provider/refresh unavailable retryable; revoked/insufficient scope non-retryable
observation_only_preflight: yes
fakeable_without_real_credentials_or_network: yes
cross_component_or_runtime_expansion: none found
required_tests_complete: no, expiry lifecycle coverage is incomplete and misleadingly named

CI_EVIDENCE:
workflow: Component CI
run_id: 29456659157
run_number: 2026
exact_sha: cf86aba890df6dcfb95f1d285677ceacf96a7772
conclusion: success
checks:
- cargo fmt: success
- cargo check: success
- cargo test: success
- cargo clippy: success
- Finalize CI diagnostics: success
- Upload CI diagnostics: skipped because no failures
ci_interpretation: CI proves current code compiles, formats, tests and lints, but the existing test suite does not exercise the defective post-refresh expiry boundary.

PR_STATE:
open: yes
draft: yes
merged: no
mergeable: yes

CHANGES_BY_REVIEWER:
product_code_changed: no
tests_changed: no
workflow_changed: no
report_only_changed: yes
ci_skip_used: yes
ci_skip_reason: clean-review report-only commit

BLOCKERS:
- Focused fixer required for deterministic time handling, post-refresh minimum-lifetime validation and exact lifecycle tests.

NEXT_RECOMMENDED_AGENT:
fixer-worker

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Exact SHA cf86aba890df6dcfb95f1d285677ceacf96a7772 is green but can return a refreshed access token that is unusable relative to the caller-provided time boundary. The required deterministic bounded expiry lifecycle and tests are incomplete. No deployment or merge-readiness claim is made.

PUSHED:
yes
