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
head: 65b0839f2a2e4213e4d34c082096e5a418c8fbd9
branch: claude/portal-rs-t45-privacy
pr: 1766
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/characters/src/profile
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/stats/src/privacy.rs
proven:
  - issue 277 open; no conflicting live PR found
  - final fmt/clippy/workspace tests pass at the material implementation head
  - four privacy browser projects pass; migration down/up passes
derived: []
unknown:
  - Game acceptance/producer/coverage evidence for G-4, G-8 and G-9
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - historical killer names cannot establish current privacy identity
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-t45-privacy.md
  - portal-rs/Cargo.lock
  - portal-rs/crates/community/src/privacy.md
  - portal-rs/crates/characters/src/profile/README.md
  - portal-rs/crates/characters/src/profile/authority.rs
  - portal-rs/crates/characters/src/profile/http.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/community/Cargo.toml
  - portal-rs/crates/community/src/deaths.rs
  - portal-rs/crates/community/src/deaths/README.md
  - portal-rs/crates/community/src/deaths/tests.rs
  - portal-rs/crates/community/src/deaths/web.rs
  - portal-rs/crates/community/src/guilds.md
  - portal-rs/crates/community/src/guilds.rs
  - portal-rs/crates/community/src/guilds_pages.rs
  - portal-rs/crates/community/src/houses.rs
  - portal-rs/crates/community/src/lib.rs
  - portal-rs/crates/community/src/online.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/src/profiles.md
  - portal-rs/crates/community/src/profiles.rs
  - portal-rs/crates/community/tests/privacy_authority.rs
  - portal-rs/crates/stats/Cargo.toml
  - portal-rs/crates/stats/src/guilds.rs
  - portal-rs/crates/stats/src/guilds/http.rs
  - portal-rs/crates/stats/src/guilds/tests.rs
  - portal-rs/crates/stats/src/lib.rs
  - portal-rs/crates/stats/src/online/README.md
  - portal-rs/crates/stats/src/privacy.rs
  - portal-rs/crates/stats/src/store.rs
  - portal-rs/crates/stats/src/world.rs
  - portal-rs/crates/stats/src/world/http.rs
  - portal-rs/crates/stats/tests/privacy.rs
  - portal-rs/e2e/specs/privacy.spec.mjs
  - portal-rs/e2e/specs/public.spec.mjs
  - portal-rs/migrations/20261107130001_t45_privacy.down.sql
  - portal-rs/migrations/20261107130001_t45_privacy.up.sql
  - portal-rs/templates/profile/en.ftl
  - portal-rs/templates/profile/pl.ftl
  - portal-rs/templates/profile/settings.html
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: final material tree formatted
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: PASS, exit 0, final material implementation
  - command: Playwright privacy.spec.mjs (PL/EN, 1440/390)
    result: PASS
    evidence: 4 passed, zero retries, 47.1 seconds
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: 1101 passed, 0 failed, 3 ignored; fresh PostgreSQL 18.6, locks=1024, connections=300, threads=4
blockers:
  - Game public projection qualification and G-9 share contract
next_action: land PR 1766; Game G-4/G-8 acceptance gates remain owner decisions
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: same-repository PR 1766 is open; disposition becomes terminal at merge
source_branch_evidence: PR 1766 for governing Issue 277
```

Publication: owner-authorized new branch push only; no PR, merge or deployment.
Detailed files, qualification evidence and exact Game blockers: `portal-rs/crates/community/src/privacy.md`.
