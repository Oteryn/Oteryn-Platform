---
task_id: OTERYN-20261115-a3-02-identity
governing_issue: 536
required_reads:
  - docs/agents/GOVERNANCE_CONTRACT.json
search_first: []
optional_reads: []
---

# Audit wave 3: identity

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN, verified live 2026-11-15). Owner supplies exact findings, base, branch, validation and push instructions; no PR or merge. Single agent, no delegation. Live open happy-davinci PR search returned no competing ownership; the parallel RBAC task's excluded files remain untouched. Jira synchronization stays with the coordinator.

Base: 90234d8d2d0a51a654b8ff6947d38372a740acba.
Branch: claude/happy-davinci-ijz3wy-fix-a3-02-identity.

All four findings confirmed against the base. W3-04: bind browser mutations to their live session under the account lock. W3-01: retain producer terminal status outside the bounded data queue. W3-02: resolve trusted-proxy client addresses before charging credential policies. W3-03: derive challenge refusal language from its route. No migrations, contract-byte changes or feature-flag changes.

Owned paths: portal-rs/crates/identity; portal-rs/crates/web/src/lib.rs; portal-rs/crates/web/src/routes/verify.rs; this packet. The web files are required browser callers for consents and verification resend.

Validation is complete: 418 tests executed and passed (identity 170, portal 189, web 59), with zero failures. The normal suites passed 416 and marked two pre-existing browser cases ignored; both were then explicitly executed and passed with real Chromium and axe. No tests remain unexecuted. All four focused PostgreSQL regressions pass and each failed with its original affected implementation restored. The language regression checks the full application message and the adapter retry link under real session/CSRF middleware, because the shared error shell replaces module links.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:41:22Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-02-identity
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/identity
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/verify.rs
proven:
  - All four findings and every listed additional mutation path are present on the admitted base
  - Every finding has an executed before-fix PostgreSQL failure and a passing focused regression
  - All eleven browser mutations check live session ownership and both expiries under the account lock
  - Formatting and warnings-denied Clippy pass on Rust 1.97 for all three affected crates
  - All 418 tests executed and passed, including both pre-existing ignored Chromium and axe cases
  - No migrations, contract bytes, feature flags, dependencies or RBAC-owned files changed
unknown:
  - Coordinator integration disposition and Jira synchronization
derived: []
conflicts: []
first_failure:
  marker: none
  evidence: Expected before-fix failures retained in local baseline logs
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261115-a3-02-identity.md
  - portal-rs/crates/identity/src/abuse/http.rs
  - portal-rs/crates/identity/src/abuse/tests.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/consents.rs
  - portal-rs/crates/identity/src/consents_http.rs
  - portal-rs/crates/identity/src/credentials_http.rs
  - portal-rs/crates/identity/src/credentials_http_tests.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_http.rs
  - portal-rs/crates/identity/src/export.rs
  - portal-rs/crates/identity/src/export_tests.rs
  - portal-rs/crates/identity/src/lib.rs
  - portal-rs/crates/identity/src/mfa.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mutation_actor.rs
  - portal-rs/crates/identity/src/privacy.rs
  - portal-rs/crates/identity/src/privacy_web.rs
  - portal-rs/crates/identity/src/recovery_key.rs
  - portal-rs/crates/identity/src/recovery_key_http.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/verify.rs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Repository-pinned Rust 1.97, no tool installation
  - command: cargo clippy --locked --all-targets -p oteryn-identity -p oteryn-web -p oteryn-portal -- -D warnings
    result: PASS
    evidence: Final implementation and regression fixtures
  - command: cargo test --locked --no-fail-fast -p oteryn-identity -p oteryn-portal audit3 -- --nocapture --test-threads=1
    result: PASS
    evidence: Four PostgreSQL regressions passed; language 2.002s, export terminal error 2.471s, revoked-session confirmation 4.000s, trusted-proxy budget 2.514s
  - command: cargo test --locked -p oteryn-identity -p oteryn-web -p oteryn-portal -- --nocapture --test-threads=1
    result: PASS
    evidence: 416 passed, zero failed; two pre-existing ignored browser tests explicitly executed below; no PostgreSQL skip notices
  - command: cargo test --locked -p oteryn-identity -p oteryn-web -p oteryn-portal -- --ignored --nocapture --test-threads=1
    result: PASS
    evidence: Both existing ignored tests passed with real Chromium, Playwright and axe; dummy checkout 6.121s, bilingual keyboard and accessibility search 12.471s
  - command: Before-fix and final PostgreSQL regression evidence
    result: PASS
    evidence: Final run language 1.708s, terminal export error 2.166s, revoked-session lock race 3.427s, trusted-proxy budget 2.728s; production 30-second export deadline 31.728s; each finding failed against the original affected implementation
  - command: Product actor-to-result validation
    result: PASS
    evidence: Real browser-session and CSRF dispatch with PostgreSQL covers the blocked confirmation race, proxy budget, English refusal, existing recovery/MFA/privacy/consent/email/verification flows; Chromium exercises both existing browser cases
blockers: []
next_action: Publish the requested source branch for coordinator review; no PR or merge authorized
```

The checkpoint head records the admitted base; validation covers the final candidate. Publication reports the resulting SHA without a self-referential packet SHA. The owner requests a branch push without a PR, so no PR exact-head CI or merge readiness is claimed. Account-only service calls retain their trusted internal API; every listed browser caller supplies the session binding. PostgreSQL fixture suites run serially to fit the local lock table; concurrency scenarios inside individual tests remain enabled.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner-requested branch for coordinator review; no PR or merge requested
source_branch_evidence: Publication SHA and push result are provided in the final task receipt; no PR requested
```

Browser validation reused installed native Node and Chromium plus prepared Playwright. Missing real axe JavaScript test libraries (4.14.0) were integrity-verified and staged only under /tmp; no tool was reinstalled and no repository dependency or lock changed.
