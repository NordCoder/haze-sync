# W1-FIX-GDA-GDA-P3-REVIEW

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P3 OAuth Expiry`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P3-REVIEW`

This is a focused review-fixer for the existing `GDA-GDA-P3-LIVE-GOOGLE-OAUTH` phase. Do not begin another GDrive product phase.

Review target:
- code-bearing SHA: `cf86aba890df6dcfb95f1d285677ceacf96a7772`;
- clean-review report blob: `ec281ebc5f3f5343887d339d7f676977587ab738`;
- green CI run: `29456659157`, number `2026`.

The review found one blocking lifecycle defect:
- token construction validates expiry against hidden wall-clock `SystemTime::now()`;
- `GoogleAuthClient::access_token(now)` does not revalidate a refreshed token against its explicit caller-provided `now` before caching/returning it;
- a refreshed token can therefore be returned below the required minimum lifetime for the caller's time boundary.

Required fix:
1. Make access-token construction/validation deterministic with an explicit reference time, or otherwise remove hidden wall-clock validation from the model.
2. After refresh, validate the returned token against the same `now` passed to `access_token(now)` before storing or returning it.
3. Reject a refreshed token below `MIN_TOKEN_LIFETIME` as the accepted safe `RefreshUnavailable` category without exposing provider details.
4. Add deterministic tests for:
   - an expired cached token causing refresh;
   - a refreshed token usable for the caller-provided time being cached/returned;
   - a refreshed token below the minimum lifetime being rejected and not cached;
   - existing valid cached-token behavior remaining unchanged.
5. Preserve the read-only credential-file contract, memory-only token lifecycle, auth/scope/provider categories, retry classification, observation-only preflight, redaction and fake-only tests.

Allowed scope:
- `crates/haze-gdrive-adapter/src/auth.rs`;
- focused GDrive auth tests;
- minimal docs only if an internal timing contract must be clarified;
- control report.

Forbidden:
- real credentials or network calls;
- credential-file writes;
- Server/API/Storage/scheduler/Deployment/sibling/workflow changes;
- unrelated refactors or test weakening;
- merge, rebase, force-push or PR draft-state changes.

Create code/test changes without CI skip. Obtain a new full exact-SHA Component CI run with fmt/check/test/clippy and diagnostics finalization green.

Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-GDA-GDA-P3-REVIEW`;
- `chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P3 OAuth Expiry`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record exact changed paths, deterministic time model, post-refresh validation, lifecycle tests, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT; a repeat OAuth/security clean review follows.
