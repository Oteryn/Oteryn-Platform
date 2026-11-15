---
task_id: OTERYN-20261115-a2-13-world-atlas-wiki
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2 world, atlas and wiki

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open, live-verified 2026-11-15). User specifies exact base, branch, seven findings, tests and push; no PR. Single agent and writer. Jira synchronization remains with coordinator.

Base: c6fdc5438a4afa91f32d547b7cfc61f22bf21139. Branch: claude/happy-davinci-ijz3wy-fix-a2-13-world-atlas-wiki.

Owned paths: portal-rs world, atlas, wiki; shared wiki entity template; additive migration; Cargo.lock for wiki's existing atlas dependency; this packet. No overlap found in live open task PR search or existing active audit packet (Bazaar/characters).

Acceptance: confirm and fix W2-01 through W2-07; regression tests, real local PostgreSQL execution with durations, pinned Rust fmt/clippy/tests; one requested authored commit and non-force push. No contract wire changes, workflow changes, game repository access or production operations.

Status: ready for requested branch publication; validation completed. Source branch disposition: retain for coordinator review; no merge or PR requested.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:25:43Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-13-world-atlas-wiki
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/world/
  - portal-rs/crates/atlas/
  - portal-rs/crates/wiki/
  - portal-rs/templates/wiki/entity.html
  - portal-rs/migrations/20261115120014_fix_a2-13-world-atlas-wiki_authority.sql
  - portal-rs/Cargo.lock
  - docs/agents/tasks/active/OTERYN-20261115-a2-13-world-atlas-wiki.md
proven:
  - Exact fetched base matches user pin
  - All seven findings confirmed against the base
  - Prepared startup completed; PostgreSQL 18.6 available
  - Rust 1.97 installed and used by removing environment override
  - Live repository identity Oteryn/Oteryn-Platform and governing Issue 536 verified
derived: []
unknown:
  - Remote publication readback pending
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/world/
  - portal-rs/crates/atlas/
  - portal-rs/crates/wiki/
  - portal-rs/templates/wiki/entity.html
  - portal-rs/migrations/20261115120014_fix_a2-13-world-atlas-wiki_authority.sql
  - portal-rs/Cargo.lock
  - docs/agents/tasks/active/OTERYN-20261115-a2-13-world-atlas-wiki.md
validation:
  - command: cargo test --locked -p oteryn-world -p oteryn-atlas -p oteryn-wiki
    result: PASS
    evidence: 129 passed, 0 failed, 0 ignored; real PostgreSQL regressions executed; two pre-existing optional asset/game smoke checks self-reported unavailable fixtures
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97, final material tree
  - command: cargo clippy --locked --all-targets -p oteryn-world -p oteryn-atlas -p oteryn-wiki -- -D warnings
    result: PASS
    evidence: Rust 1.97, final material tree
  - command: regression sensitivity with base behavior restored
    result: PASS
    evidence: eight expected failing regressions plus corrected entity-pagination regression failed with its base continuation logic; repaired tree passes every regression
blockers: []
next_action: Commit the reviewed validated tree, preserve exact-candidate recovery bundle, push only the requested branch and verify its remote SHA
```

## Repair and execution evidence

- W2-01: persisted configured authority reservation before source I/O; retired epochs and snapshots checked under publication singleton; migration seeds existing history; explicit exact-target/profile operator recovery records immutable actor/evidence/scope/rollback history. Public reads recheck accepted authority.
- W2-02: recursively check PostgreSQL JSONB compatibility for entities and relation payloads, withholding only the affected complete category while retaining original artifact bytes.
- W2-03: acquire both resource singleton locks before 71010 and account share locks, bound resource waits to 250 ms, then recheck authorization. Actual unrelated role revocation completes within a 150 ms deadline while each singleton is held.
- W2-04: bounded guide and revision-history lookahead; exact-twelve and exact-ten mounted HTTP regressions.
- W2-05: wiki uses atlas's shared flag decision and encoded item/creature links; actual detail-to-delivery navigation tested through merged router.
- W2-06: shared canonical kind mapping for imports, readers and approvals; explicit backward-compatible alias copies preserve old rows and deny conflicts; canonical creature/NPC markers followed through merged router.
- W2-07: request-wide 4096 aggregate estimate-term budget, request-local repeated-estimate cache, one spawn index, two admitted requests per service, owned permit retained across blocking computation, heartbeat regression on current-thread runtime.

Final new PostgreSQL regression durations (seconds):

- retired_epoch_delayed_refresh_cannot_replace_publication: 1.142585
- paused_first_epoch_refresh_is_fenced_before_its_first_publication: 1.143246
- retired_epoch_requires_exact_audited_recovery: 1.055454
- nul_opaque_category_does_not_poison_valid_npc_publication: 1.102711
- publication_wait_does_not_hold_global_authorization_fence: 2.115188
- entity_router_exact_twelve_guides_has_no_next_page: 0.986719
- history_exact_ten_snapshots_has_no_older_control: 1.171308
- wiki_item_creature_map_actions_follow_delivery_selectors: 1.186165
- canonical_atlas_markers_follow_wiki_through_merged_router: 1.187743
- migration_copies_aliases_and_denies_conflicting_approvals: 0.974943

All are greater than 0.05 seconds; no new PostgreSQL regression skipped. The pre-existing optional real-sprite and full-game-tree smoke checks lacked their fixtures and are not claimed as executed. No game repository was accessed. Initial parallel migrations exhausted local PostgreSQL shared locks; final validation uses one test thread and two build jobs. Final rollback regression passes with audit provenance independent of candidate-table retention.

GitHub authentication briefly failed during read-only checks, then recovered before publication. Normal non-force publication follows the user's exact branch and command; no PR or other branch is authorized. The packet's head records the admitted input commit; final output SHA and push/readback evidence belong to the execution closeout.
