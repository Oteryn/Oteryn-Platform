---
task_id: OTERYN-20261117-bazaar-scan-bound
governing_issue: 1919
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-bazaar-scan-bound

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1919.

Diagnose why `issue()` read 269 tuples for `Return` in
`postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait` when the
setup commits one transaction per receipt, and make the test guard the
production row layout without loosening the `< 100` bound.

## Diagnosis

Every query `issue()` runs for `Return` was captured with `auto_explain`
(analyze, buffers, nested statements) inside the measured transaction on
PostgreSQL 18.7, under both setup layouts. All read one row through an
index except two plans:

- `retained()` runs `UPDATE bazaar_disposition_fences ... WHERE operation_id=$1 AND NOT quarantined`.
  With one transaction per receipt, this is a **Seq Scan over 256 rows that removes 255 by filter**. This accounts for 256 of the 269 tuples.
  With the single setup transaction, it is an Index Scan on `bazaar_disposition_fences_pkey` that reads 1 row.
- `UPDATE bazaar_cleanup_attempts ... WHERE listing_id,command,root_step` is a Seq Scan over the 4-row table in both layouts.

Cause: physical layout plus the planner's small-table cost rule. Nothing in
the query scales with receipt history. In the single transaction, each fence
row is inserted and then updated in the same transaction. The dead versions
leave the heap at 6 pages, so the PK probe (cost 0.27..8.29) is cheaper. With
one transaction per receipt, pruning and autovacuum keep the 256 fences in
3 pages (relpages 3, n_dead_tup 0, last_autovacuum set). The whole-heap
Seq Scan (cost 6.20) then costs less than one PK probe, and the planner picks it.
`VACUUM (ANALYZE)` after setup would not help: it produces the compact
3-page heap that triggers the Seq Scan. On the leftover schema after
autovacuum, `EXPLAIN (ANALYZE, BUFFERS)` of the same UPDATE still chose
the Seq Scan (Rows Removed by Filter: 255, shared hit=6).

Why production is bounded: PostgreSQL chooses that Seq Scan only while
reading the whole heap costs less than one primary-key probe (about 8.3 cost
units at default `random_page_cost=4`, `seq_page_cost=1`). That holds
only for a heap of a few pages, about 3 pages here or about 260 rows. Above
that the PK index wins. So the read is bounded by a constant and does not
grow with history. No query or index change is needed. Every predicate
already has an index path (the fences UPDATE uses the PK). No migration was
added.

Sanitized plan summary (scan nodes only; schema name replaced, timings informational):

### one transaction per receipt (production shape), ANALYZE only
- `SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE s…`
  - -> Index Scan using bazaar_game_issuer_pkey on bazaar_game_issuer (actual time=0.024..0.026 rows=1.00 loops=1)
- `INSERT INTO bazaar_cleanup_attempts(listing_id,command,root_step,current_step) VALUES($1,$2,$3,$3) ON CONFLICT…`

- `SELECT intent FROM bazaar_game_outbox WHERE issuance_key=$1`
  - Index Scan using bazaar_game_outbox_issuance_key_key on bazaar_game_outbox (actual time=0.010..0.011 rows=1.00 loops=1)
- `SELECT o.intent,o.binding_digest,r.receipt,r.receipt_digest,(SELECT d.payload FROM bazaar_authority_decisions …`
  - -> Index Scan using bazaar_game_outbox_pkey on bazaar_game_outbox o (actual time=0.009..0.009 rows=1.00 loops=1)
  - -> Index Scan using bazaar_authority_receipts_pkey on bazaar_authority_receipts r (actual time=0.008..0.009 rows=1.00 loops=1)
  - -> Bitmap Heap Scan on bazaar_authority_conflicts c (actual time=0.005..0.005 rows=0.00 loops=1)
  - -> Bitmap Index Scan on bazaar_conflict_operation (actual time=0.001..0.001 rows=0.00 loops=1)
  - -> Index Only Scan using bazaar_decision_operation_generation on bazaar_authority_decisions d (actual time=0.013..0.013 rows=1.00 loops=1)
  - Heap Fetches: 1
  - -> Index Scan using bazaar_decision_operation_generation on bazaar_authority_decisions d_1 (actual time=0.004..0.004 rows=1.00 loops=1)
- `UPDATE bazaar_disposition_fences SET no_effect_generation=$2::text::numeric,terminal_committed=$3 WHERE operat…`
  - -> Seq Scan on bazaar_disposition_fences (actual time=0.024..0.032 rows=1.00 loops=1)
  - Rows Removed by Filter: 255
- `SELECT 1 FROM ONLY "<schema>"."bazaar_listings" x WHERE "id" OPERATOR(pg_catalog.=) $1 FOR KEY SHARE OF x`
  - -> Index Scan using bazaar_listings_pkey on bazaar_listings x (actual time=0.004..0.004 rows=1.00 loops=1)
- `UPDATE bazaar_cleanup_attempts SET current_step=$4 WHERE listing_id=$1 AND command=$2 AND root_step=$3`
  - -> Seq Scan on bazaar_cleanup_attempts (actual time=0.005..0.006 rows=1.00 loops=1)
  - Rows Removed by Filter: 3
- `SELECT intent FROM bazaar_game_outbox WHERE issuance_key=$1`
  - Index Scan using bazaar_game_outbox_issuance_key_key on bazaar_game_outbox (actual time=0.007..0.008 rows=1.00 loops=1)

### single setup transaction (main), ANALYZE only
- `SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE s…`
  - -> Index Scan using bazaar_game_issuer_pkey on bazaar_game_issuer (actual time=0.007..0.007 rows=1.00 loops=1)
- `INSERT INTO bazaar_cleanup_attempts(listing_id,command,root_step,current_step) VALUES($1,$2,$3,$3) ON CONFLICT…`

- `SELECT intent FROM bazaar_game_outbox WHERE issuance_key=$1`
  - Index Scan using bazaar_game_outbox_issuance_key_key on bazaar_game_outbox (actual time=0.030..0.030 rows=1.00 loops=1)
- `SELECT o.intent,o.binding_digest,r.receipt,r.receipt_digest,(SELECT d.payload FROM bazaar_authority_decisions …`
  - -> Index Scan using bazaar_game_outbox_pkey on bazaar_game_outbox o (actual time=0.005..0.005 rows=1.00 loops=1)
  - -> Index Scan using bazaar_authority_receipts_pkey on bazaar_authority_receipts r (actual time=0.007..0.008 rows=1.00 loops=1)
  - -> Bitmap Heap Scan on bazaar_authority_conflicts c (actual time=0.004..0.004 rows=0.00 loops=1)
  - -> Bitmap Index Scan on bazaar_conflict_operation (actual time=0.001..0.001 rows=0.00 loops=1)
  - -> Index Only Scan using bazaar_decision_operation_generation on bazaar_authority_decisions d (actual time=0.009..0.009 rows=1.00 loops=1)
  - Heap Fetches: 1
  - -> Index Scan using bazaar_decision_operation_generation on bazaar_authority_decisions d_1 (actual time=0.002..0.002 rows=1.00 loops=1)
- `UPDATE bazaar_disposition_fences SET no_effect_generation=$2::text::numeric,terminal_committed=$3 WHERE operat…`
  - -> Index Scan using bazaar_disposition_fences_pkey on bazaar_disposition_fences (actual time=0.013..0.015 rows=1.00 loops=1)
- `SELECT 1 FROM ONLY "<schema>"."bazaar_listings" x WHERE "id" OPERATOR(pg_catalog.=) $1 FOR KEY SHARE OF x`
  - -> Index Scan using bazaar_listings_pkey on bazaar_listings x (actual time=0.003..0.003 rows=1.00 loops=1)
- `UPDATE bazaar_cleanup_attempts SET current_step=$4 WHERE listing_id=$1 AND command=$2 AND root_step=$3`
  - -> Seq Scan on bazaar_cleanup_attempts (actual time=0.005..0.006 rows=1.00 loops=1)
  - Rows Removed by Filter: 3
- `SELECT intent FROM bazaar_game_outbox WHERE issuance_key=$1`
  - Index Scan using bazaar_game_outbox_issuance_key_key on bazaar_game_outbox (actual time=0.005..0.006 rows=1.00 loops=1)

## Change

The test setup now matches production (`authority_http.rs`): each intent,
dispatch and `receive()` commits in its own transaction, so each receive
has its own mutation budget, the #1920 rule. The per-receive
`oteryn.mutation_deadline` reset is no longer needed and is removed. The cleanup
pointer and pending intent are committed in one more transaction per command.
The measured transaction sets `SET LOCAL enable_seqscan=off` before the
counter snapshot. In PostgreSQL this only disfavours a Seq Scan when another
path exists. A query with no index path still seq-scans and is counted. The
`< 100` bound and every assertion are unchanged.

## Acceptance criteria

- [x] Cause of the 269 tuples identified with plans under both layouts
- [x] Test setup commits one transaction per receipt (production shape), each receive with its own budget
- [x] Bound `< 100` unchanged; test passes with the production layout
- [x] Red without the change: production layout without `enable_seqscan=off` fails `Return: 269 tuples read`
- [x] Guard still catches an unindexed lookup: with `enable_seqscan=off` kept and the fences UPDATE predicate made unindexable (`operation_id::text=$1::text`, temporary), the test fails `Return: 266 tuples read`
- [ ] Portal RS CI green on the PR head (no PR yet)

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-scan-bound.md
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
modules:
  - bazaar
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T10:40:00Z
head: 20d4f17d0945834f463193ff5d689f0fdcc932d2
branch: claude/happy-davinci-ijz3wy-bazaar-scan-bound
pr: none
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-scan-bound.md
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
proven:
  - 256 of the 269 tuples are one Seq Scan of bazaar_disposition_fences in retained() UPDATE (auto_explain, PostgreSQL 18.7)
  - Single-transaction layout heap is 6 pages and uses the PK index; per-receipt layout heap is 3 pages and seq-scans
  - Every issue() query for Return has an index path; no production query or index change needed; no migration added
derived:
  - The Seq Scan choice is capped by the planner crossover (heap cheaper than one PK probe), so the read does not grow with history
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Missing index or a lookup that scales with receipt history (every plan is an index probe except two small-table Seq Scans)
  - VACUUM (ANALYZE) after setup restores the bound (vacuum yields the compact 3-page heap that triggers the Seq Scan)
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-scan-bound.md
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
validation:
  - command: cargo test -p oteryn-bazaar --lib postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait (PostgreSQL 18.7, production layout, without enable_seqscan=off)
    result: FAIL
    evidence: intended red, Return 269 tuples read
  - command: same test with enable_seqscan=off and the fences UPDATE made unindexable (temporary)
    result: FAIL
    evidence: intended red, Return 266 tuples read
  - command: same test, final change, 5 runs on an idle machine
    result: PASS
    evidence: 4.43 s, 4.19 s, 5.68 s, 5.24 s, 4.25 s
  - command: cargo fmt --all --check; cargo clippy -p oteryn-bazaar --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: cargo test -p oteryn-bazaar --lib (PostgreSQL 18.7, max_locks_per_transaction=1024 and max_connections=300 as in portal-rs-ci.yml)
    result: FAIL
    evidence: 116 passed, 1 failed; 62 postgres_ tests ran; the failure is runtime::tls postgres_exchange_deadline_cancels_issuer_wait_and_releases_pool (Elapsed), which fails the same way on untouched main 20d4f17d (2 of 2) in this container and is outside owned paths
blockers:
  - none
next_action: coordinator opens the PR; Portal RS CI confirms the tls deadline test on CI PostgreSQL
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Task branch for the issue 1919 test change; retained until merge
source_branch_evidence: The PR records the published head
```
