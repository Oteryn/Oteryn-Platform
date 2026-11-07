---
task_id: OTERYN-20261107-portal-rs-t45-privacy
governing_issue: 277
required_reads:
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
search_first: []
optional_reads: []
---

# T45 final privacy authority

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/277 (open, read 2026-11-07).
Owner instruction: implement Rust T45, push new branch, no PR, workflows or deployment.
Base: origin/main 3d6a1ae2. Jira mapping unknown; synchronization pending.

## Ownership

Owned: portal-rs character profile, community privacy wiring, stats privacy adapters,
profile templates, additive migrations 20261107130001, focused tests and E2E specs.
Concurrent world/atlas/wiki entities/stats world/local.mjs excluded; only necessary
stats/world/http.rs privacy wiring allowed. No live PR claims these paths in preflight.

## Acceptance

- Owner-only transactional audited, CSRF protected and rate limited preferences.
- Saved, current unique ownership and account bounds; unknown/missing/error deny.
- Real production privacy adapters, accepted Game gates retained.
- PostgreSQL 18 tests and requested fmt/clippy/workspace tests.
- Report G-4/G-8 and T81/G-9 blockers; no fabricated data.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T13:00:00Z
head: UNKNOWN
branch: codex/portal-rs-t45-privacy
pr: none
status: validating
context_routes: []
owned_paths:
  - portal-rs/crates/characters/src/profile
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/stats/src/privacy.rs
proven:
  - issue 277 open; no conflicting live PR found
derived: []
unknown:
  - Game acceptance/producer/coverage evidence for G-4, G-8 and G-9
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - historical killer names cannot establish current privacy identity
changed_paths: []
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: final material tree formatted
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: passed; final exact command running again with workspace acceptance
  - command: Playwright privacy.spec.mjs (PL/EN, 1440/390)
    result: PASS
    evidence: 4 passed, zero retries, 47.1 seconds
  - command: cargo test --locked --no-fail-fast
    result: NOT_RUN
    evidence: final run in progress on fresh migrated PostgreSQL 18.6, threads=4
blockers:
  - Game public projection qualification and G-9 share contract
next_action: finish final workspace verification and push the requested branch
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requests pushed branch without PR; owner reviews this branch after delivery
source_branch_evidence: user task instruction
```
