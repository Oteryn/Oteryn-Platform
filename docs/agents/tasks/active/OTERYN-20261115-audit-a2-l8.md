---
task_id: OTERYN-20261115-audit-a2-l8
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2 L8 repairs

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536,
verified open at admission and before publication. Owner requests one writer,
base c6fdc5438a4afa91f32d547b7cfc61f22bf21139, branch
claude/happy-davinci-ijz3wy-fix-a2-l8, regression tests and a branch push;
no PR, merge or deployment. Repository identity was verified live as
Oteryn/Oteryn-Platform. No subagents were used.

All six findings were confirmed on that base. No finding was dismissed.

- L8-001: current transfer permission follows payload binding on transfers,
  offers and settlements, including exact replay. Stored receipts and entries
  retain their original identity/content.
- L8-002: a separately gated G-9 revision-3 codec, admitted stream generations,
  current ownership/restore/deletion fences, durable deduplication, stored and
  applied prefixes, and pending/read/receipt exchanges. G-10 bytes and limits
  remain unchanged. The binary selects record codecs by verified peer purpose.
- L8-004: guild ingest/watermarks and world-daily serving require the parent
  public-projections flag in addition to their child switches.
- L8-005: both dispatchers reuse peer/configuration admission without polling
  the body, and dispatch failures retain no-store headers. The shared endpoint
  admits only when a configured family can accept the peer. The wire's only
  family discriminator is inside the body; the selected child guard runs again
  after selection. No new routing header or wire member was invented.
- L8-007: all listed writers persist one content-free conflicting ordered
  reference per family/subject, preserving the first snapshot. Serving refuses
  quarantined evidence and completeness cannot advance. Death occurrence
  conflicts are covered as well as snapshot conflicts. Independent families
  continue; retries and epoch raises cannot remove the fence.
- L8-008: skills revalidate body and profile eligibility after asynchronous
  privacy authorization, under the shared projection transaction fence.

G-9 requires trusted generation and catalogue admission; requests cannot
establish those inputs. It activates no hunt/drop reads, party capture,
sharing, export or aggregation. All game/money gates remain default off and
production remains refused. Contract and workflow files are unchanged.

## Regression evidence

Before-fix behavior was restored temporarily without weakening any regression.
The expected failures were observed for every finding, then candidate sources
were restored. Wallet replay failed on TransferBlocked; skills returned old
values; equipment completeness advanced; guild publication accepted the parent
off; daily serving remained enabled; both dispatchers polled a failing body;
and the binary refused the accepted G-9 golden. PostgreSQL failure runs lasted
0.15--1.08 seconds. A compile-only failure during the temporary binary rollback
was corrected and the binary behavior failure was then reproduced.

Final new PostgreSQL test durations (all exceed 0.05 seconds):

| Test | Seconds |
| --- | ---: |
| exact_replays_revalidate_current_transfer_permission | 0.995 |
| db_g9_revision3_goldens_through_binary_router | 1.015 |
| postgres_configured_guild_parent_off_refuses_publication_and_watermark | 0.935 |
| configured_parent_off_refuses_fresh_retained_daily_data | 0.148 |
| postgres_equipment_conflict_quarantines_owner_and_stalls_completeness | 1.084 |
| postgres_projection_conflict_stalls_statistics_and_preserves_other_family | 0.070 |
| postgres_skills_suppression_during_privacy_denies_old_snapshot | 1.115 |

Non-PG polling regressions:
`disabled_and_wrong_peer_refuse_before_body_poll` and
`watermark_dispatch_refuses_before_polling_disabled_or_wrong_peer` pass for
both a failing stream and an oversized body. Original guild, house, death and
legacy ingest regressions now additionally assert quarantine; explicit fixture
reconciliation permits their independent existing assertions to continue.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:49:27Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-l8
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/README.md
  - portal-rs/bin/oteryn-portal/
  - portal-rs/crates/community/
  - portal-rs/crates/internal-api/
  - portal-rs/crates/playerdata/
  - portal-rs/crates/stats/
  - portal-rs/crates/wallet/
  - portal-rs/Cargo.lock
  - portal-rs/migrations/20261115120019_fix_a2-l8_projection_conflicts.sql
proven:
  - All six findings reproduced on the admitted base behavior and repaired
  - Current permission is required for all three wallet mutation replays
  - G-9 goldens traverse the binary router and real PostgreSQL
  - Current flags and quarantine fences prevent affected serving and completion
  - Skills suppression during asynchronous privacy cannot disclose old values
  - No contract, workflow, production or external Game repository changes
  - Exact Rust 1.97.0 pin used without reinstalling tools
  - Final required test run completed with real PostgreSQL and no PG skips
  - Live repository identity and governing issue checked before publication
derived:
  - Shared watermark admission precedes the body-only family discriminator
unknown:
  - Programme coordinator Jira mapping and synchronization
conflicts: []
first_failure:
  marker: none
  evidence: Expected before-fix regression failures resolved; local fixed-name test schema residue cleaned
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261115-audit-a2-l8.md
  - portal-rs/Cargo.lock
  - portal-rs/README.md
  - portal-rs/bin/oteryn-portal/Cargo.toml
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_tests.rs
  - portal-rs/crates/community/Cargo.toml
  - portal-rs/crates/community/src/deaths/storage.rs
  - portal-rs/crates/community/src/deaths/tests.rs
  - portal-rs/crates/community/src/deaths/web.rs
  - portal-rs/crates/community/src/families/storage.rs
  - portal-rs/crates/community/src/families/tests.rs
  - portal-rs/crates/community/src/families/web.rs
  - portal-rs/crates/community/src/guilds.rs
  - portal-rs/crates/community/src/guilds_pages.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/houses.rs
  - portal-rs/crates/community/src/houses_ingest.rs
  - portal-rs/crates/community/src/houses_tests.rs
  - portal-rs/crates/community/src/ingest.rs
  - portal-rs/crates/community/src/ingest/ingest_tests.rs
  - portal-rs/crates/community/src/online.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/src/profiles.rs
  - portal-rs/crates/community/src/profiles_tests.rs
  - portal-rs/crates/community/src/rankings.rs
  - portal-rs/crates/community/src/today.rs
  - portal-rs/crates/community/src/v2_watermark.rs
  - portal-rs/crates/community/tests/online.rs
  - portal-rs/crates/community/tests/privacy_authority.rs
  - portal-rs/crates/community/tests/rankings.rs
  - portal-rs/crates/community/tests/today_contract.rs
  - portal-rs/crates/internal-api/src/guard.rs
  - portal-rs/crates/internal-api/src/playerdata.rs
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/journal.rs
  - portal-rs/crates/playerdata/src/lib.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/http.rs
  - portal-rs/crates/stats/Cargo.toml
  - portal-rs/crates/stats/src/online/storage.rs
  - portal-rs/crates/stats/tests/online.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/tests/postgres.rs
  - portal-rs/migrations/20261115120019_fix_a2-l8_projection_conflicts.sql
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Final Rust 1.97.0 candidate
  - command: cargo clippy --locked --all-targets -p oteryn-wallet -p oteryn-community -p oteryn-internal-api -p oteryn-playerdata -p oteryn-stats -p oteryn-portal -p oteryn-db -- -D warnings
    result: PASS
    evidence: Every touched crate and the migration owner
  - command: cargo test --locked -p oteryn-wallet -p oteryn-community -p oteryn-internal-api -p oteryn-playerdata -p oteryn-stats -p oteryn-portal -p oteryn-db -- --nocapture --test-threads=1
    result: PASS
    evidence: 563 passed, zero failed; one pre-existing Chromium test ignored; no PG skips; seven new PG durations recorded above
  - command: Browser product E2E
    result: NOT_APPLICABLE
    evidence: Internal wire, ledger and projection-fence changes; real router and PostgreSQL actor-to-result regressions cover the affected paths
blockers: []
next_action: Push the owner-requested branch for coordinator review without creating a PR
```

The checkpoint head is the admitted base; the publication SHA is reported by
Git rather than embedded self-referentially. Required checks cover the final
candidate derived from it. No PR exact-head gate or merge qualification is
claimed for this explicitly branch-only task.

The ignored test is the pre-existing
`shop_web::tests::postgres_dummy_checkout_browser_360px`, whose base attribute
requires Chromium and browser modules. No ignore attribute was added or changed.
One earlier broader run encountered a fixed-name schema left by an earlier
failed regression; only that known local-test schema was removed before the
successful full rerun. Native psql is absent, so the already-prepared PostgreSQL
container's psql was used without installing tools or printing connection data.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner-requested fix branch retained for coordinator review; no PR or merge requested
source_branch_evidence: Branch publication and remote SHA verification are reported in the task result
```
