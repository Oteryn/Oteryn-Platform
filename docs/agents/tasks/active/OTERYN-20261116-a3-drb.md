---
task_id: OTERYN-20261116-a3-drb
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit 3: lock hierarchy, post-wait authorization and acquisition deadlines

Owner-authorized base `256a5f42`, branch
`claude/happy-davinci-ijz3wy-fix-a3-drb`. Single writer; commit and push authorized;
no PR creation, merge, deployment or live-data operation requested.
Live repository identity Oteryn/Oteryn-Platform and programme #536 OPEN verified.
Open PR inventory has no competing audit branch. Existing audit packets describe
predecessor work; this task repairs the owner's supplied delta findings.
Jira synchronization remains with the coordinator.

Owned paths: portal-rs lock/authorization implementations, their PostgreSQL tests,
RBAC lock hierarchy documentation and this evidence packet.

Final validation and red/green evidence are recorded below.

## Hierarchy and complete change inventory

The normative hierarchy and every advisory key (including migrations and test
barriers) are in `portal-rs/crates/admin/src/rbac/README.md`.
No applied migration or trigger body was edited. Trigger gate acquisition is
reentrant because application entry points acquire the same gate first.

Class A, Bazaar gate-first acquisition (paths below are relative to
`portal-rs/crates/bazaar/src/`):

| File | Changed takers |
|---|---|
| `repository.rs` | create, publish, bid, offer, accept_offer, reject_offer, cancel, expire, settle, complete_delivery |
| `items/service.rs` | create, publish, bid, offer, accept, reject, buy_now, close_auction, cancel, renew, expire, owner_view, view |
| `characters/service.rs` | own_requests, choices, request, detail, bid |
| `search.rs` | search, watch, save, saved, delete_saved |
| `moderation.rs` | seller, report, promote; lock_staff_target now takes the exclusive gate for every staff command before its target/account locks |
| `game/authority.rs` | Authority::lock, including caller-owned/ingress transactions |
| `game/coin_ports.rs` | PlatformAccounts::safe_return/state; CoinReservations::fence/account/release/settle |
| `game/game_store.rs` | Outbox::stage |
| `game/runtime/mod.rs` | configured, create, bid |
| `game/runtime/worker.rs` | tick; existing advance acquisition retained |
| `game/runtime/gold_worker.rs` | accept_gold_bid |
| `game/runtime/browser.rs` | cards, including its per-card transaction |
| `locking.rs` | protocol documentation |

Class A, resource acquisition before RBAC (also Class B):

- `admin/src/players/mod.rs`: attest_reset_evidence_impl prepares the ticket;
  request_reset prepares evidence; approve_reset prepares evidence and retains its post-fence request/expiry check.
- `admin/src/commerce/mod.rs`: submit and approve prepare the operation row in
  the canonical resource callback.
- `content/src/events/repository.rs`: save (existing target) and transition.
- `content/src/announcements.rs`: save (existing target).
- `support/src/tickets/mod.rs`: change_in prepares the ticket.
- `support/src/tickets/staff.rs`: prepare locks ticket/report/sanction and appeal
  workflow before joining RBAC.
- `support/src/sanctions/store.rs`: sanction transition prepares the sanction.
- `characters/src/bootstrap/issue.rs`: retain the shared operator fence through
  authority acquisition and check permission after the retained authority wait.
- `admin/src/bazaar.rs`: documents the exclusive gate/target/account protocol.
- `admin/src/bazaar_tests.rs`: predecessor bid fixture now acquires the shared
  gate before its manually retained listing lock; assertions/barrier unchanged.

Class B, additional post-wait revalidation:

- `admin/src/rbac/mod.rs`: durable_session_in first retains session/enrollment/
  grant rows, then evaluates database-clock lifetime and credential predicates.
- `admin/src/commerce/mod.rs`: submit rechecks after replay-row and inspect waits
  and before committing effects; approve rechecks after effects before commit.
- `admin/src/players/mod.rs`: request_reset and approve_reset revalidate after
  evidence verification; release_coin_hold revalidates after payment effects.
- `admin/src/support.rs`: execute reuses authorize_mutation after the owning
  backend's effect/audit waits, before commit.
- `store/src/catalog.rs`: activation repeats its transaction-local account and
  permission adapter after the retained pointer wait.
- `support/src/sanctions/store.rs`: transition repeats live staff authorization
  after reading the retained sanction row.
- Content's event save/transition and announcement save use the helper after
  resource acquisition, as listed above.

Class C:

- `db/src/lib.rs`: shared mutation_budget implementation, unchanged three-second
  database-clock deadline semantics.
- `admin/src/rbac/mod.rs`: re-export that implementation; existing API preserved.
- `characters/Cargo.toml`: database crate becomes a runtime dependency.
- `characters/src/bootstrap/issue.rs`: budget before its first account lock,
  refreshed before subsequent authority/fence acquisition without resetting it.

## PostgreSQL regressions

All new tests require PostgreSQL rather than silently returning when unset.
Interleavings use pg_blocking_pids tied to the held backend PID; no sleeps.

| Test | Red against predecessor implementation | Green |
|---|---|---|
| `creation_waits_for_moderation_before_locking_its_account` | PostgreSQL 55P03: creator retained account while blocked by moderation gate | PASS |
| `editorial_writers_reject_grant_expired_during_resource_wait` | announcement committed with expired durable MFA grant | PASS; announcement edit, event edit and event publish all refuse and retain original versions |
| `issuer_authority_wait_uses_shared_mutation_deadline` | outer bound elapsed while issuer still waited | PASS; zero issued intents |

The baseline was restored only for the corresponding production implementation;
test bodies and fixtures were retained. Working implementation was restored after
red runs. The expiry fixture uses a real confirmed MFA enrollment, password-bound
grant and session, and polls PostgreSQL clock_timestamp while holding an unchanged
target row. The deadline regression holds the committed bootstrap authority row.

### Complete Bazaar participant sets

The inventory also exposed a second account-order edge: a bid could retain its
actor/seller and only later acquire the earlier previous leader for hold release.
`bid_prepares_previous_leader_before_actor_account` reproduces the ordered role
writer against that real bid path. Red: PostgreSQL 55P03 on the role writer's later
account, because the bid retained it while waiting for the earlier leader.

Additional Class A takers changed:

- `bazaar/src/ports.rs`: Accounts::prepare supplies one ordered participant-set
  protocol. PlatformAccounts batches the SELECT; test/other adapters use ordered
  state reads by default.
- `bazaar/src/locking.rs`: accounts helper locks the full UUID-ordered SHARE set
  under the already-acquired moderation gate.
- `bazaar/src/repository.rs`: authorize_many, bid_in (including previous leader),
  accept_offer (selected/rejected bounded batch), reject_offer, expire and
  complete_delivery prepare complete identity sets before individual checks.
- `bazaar/src/items/service.rs`: parties and listing operations include seller,
  actor and previous leader; accept/reject/expire discover their selected/bounded
  release accounts before the first eligibility read.
- `bazaar/src/characters/service.rs`: BidAccounts::prepare forwards to the owning
  adapter so evidence checks do not split the account set.
- `bazaar/src/game/coin_ports.rs`: PlatformAccounts::prepare; lock_accounts,
  reserve and release prepare identities before issuer/wallet effects.
- `bazaar/src/game/game_store.rs`: stage prepares seller and optional buyer.
- `bazaar/src/game/authority_exchange.rs`: publish_intent prepares all parties
  before individual eligibility reads and issuer publication.
- `bazaar/src/game/runtime/mod.rs`: bid and owner action prepare the listing's
  owner, actor and previous leader before issuer acquisition.
- `bazaar/src/game/runtime/worker.rs`: advance prepares seller/leader before issuer.
- `bazaar/src/game/runtime/gold_worker.rs`: accept_gold_bid discovers its request
  buyer and retained release buyer, then prepares the complete set before issuer.
- `bazaar/src/search.rs`: transaction isolation selection precedes gate acquisition.

Financial offer batching remains bounded; participant preparation includes only
that transaction's selected/bounded release batch. No mutation retries, budget
increase, new allow or ignored tests were added.

Validation environment: PostgreSQL 18.6 (`server_version_num=180006`), Rust 1.97.0
from the repository pin. The prepared default database had a partial public schema
(wallet tables without migration records), causing 42P07 before tests could run.
A separate fresh local `portal_drb_a3` database was created; the existing database
was preserved. The test URL was supplied only through process environment, never
printed. Full Bazaar validation on the fresh database passed 130 tests before the
additional participant regression; the final combined run includes that test.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-16T03:27:16Z
head: 256a5f42e5367c049dc6b2ee7e1250b13fed9ed0
branch: claude/happy-davinci-ijz3wy-fix-a3-drb
pr: none
status: waiting
context_routes: []
owned_paths:
  - portal-rs/crates/admin/
  - portal-rs/crates/bazaar/
  - portal-rs/crates/characters/
  - portal-rs/crates/content/
  - portal-rs/crates/db/
  - portal-rs/crates/store/
  - portal-rs/crates/support/
  - docs/agents/tasks/active/OTERYN-20261116-a3-drb.md
proven:
  - PID-barrier regressions fail on predecessor implementations
  - pinned Rust workspace Clippy and formatting pass
  - PostgreSQL version is 180006
  - no applied migration edits or new allows or ignored tests
unknown:
  - none
derived: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: /tmp/drb-fmt.log
  - command: cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/drb-clippy-final.log
  - command: full touched-crate tests on PostgreSQL 18
    result: PASS
    evidence: /tmp/drb-delivery-tests.log plus /tmp/drb-store-delivery.log and explicit browser runs; 648 touched-crate tests passed
blockers:
  - none
next_action: owner or coordinator reviews the delivered task branch; no merge requested
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested branch delivery for audit review; no merge requested
source_branch_evidence: this task request and final push readback
```

### Validation corrections and final environment

- The predecessor Bazaar bid fixture now acquires the same shared gate as a real
  bid before retaining its row. Its original PID barrier and outcome assertions
  remain intact.
- The character snapshot fixture's `character_intent` uses the already captured
  database time for both Lock and Bind, matching receipt time. A later wall-clock
  second previously made Bind precede its own issuance timestamp.
- Bootstrap retains its established shared-fence serialization across authority
  acquisition (the CLI permission-revocation regression requires this). Live
  permission is checked after the authority wait, within the original deadline.
- Player logout and MFA reset approval prepare target web-session rows before
  live authorization. Self-logout remains valid because authorization precedes
  its deliberate session revocation.
- Overlapping schema-heavy validation runs exhausted the test server's shared
  lock table (SQLSTATE 53200 during DROP SCHEMA, with PostgreSQL's explicit
  max_locks_per_transaction hint). Those failed runs were stopped. The local
  PG18 test server's lock-table capacity was increased from 64 to 256 and it was
  restarted; no application setting, timeout, three-second budget or assertion
  was relaxed. Final touched-crate validation runs serially in one Cargo process.
- The final focused bootstrap CLI serialization regression passed, as did all
  seven catalog CLI tests, including both predecessor lock/fence regressions.

The final site inventory additionally includes `admin/src/players/mod.rs`
logout_all and approve_reset target-session preparation (domain resources before
live authority; actor proof rows are then reentrant), and
`bazaar/src/game/authority_tests.rs` consistent database-clock fixture time.

Final Class A namespace correction: `store/src/premium/repository.rs` mutate and
authorized_in now take account/premium-authority rows before request key hash 27,
matching callers that already retain ordered account locks. Conflicting request
reuse remains serialized by the same key and ledger uniqueness constraint.

Final gift-code Class A/B correction: `store/src/codes.rs` redeem_player uses an
unlocked admission read. Redeem_in now follows request key 55 → code row → batch
key 56 → account, then repeats active/verified eligibility under the retained
account lock before replay/effects. Internal reward redemption follows the same
resource-before-account order. The original code/batch serialization remains.
`store/src/codes_tests.rs` adds
`browser_redemption_locks_code_before_account_and_rechecks_status`: red was 55P03
because browser redemption retained the account while blocked on a code row.
Green must both permit the other writer's account lock and deny redemption after
that writer suspends the account during the held wait, with zero redemptions.
The five new regressions all use PID barriers without sleeps.

The full seven-crate run passed before this private gift-code correction. The
entire Store crate is rerun after it, together with workspace Clippy/formatting
and explicit existing Chromium tests. Other touched-crate source remained
unchanged. The final report below supersedes intermediate validation totals.

## Final validation report

All checks used repository-pinned Rust 1.97.0 and PostgreSQL 18.6 (180006).
PORTAL_TEST_DATABASE_URL was never printed. No applied migration changed.

| Check | Result | Evidence |
|---|---|---|
| cargo fmt --all --check | PASS | /tmp/drb-fmt.log |
| cargo clippy --workspace --all-targets -- -D warnings | PASS | /tmp/drb-clippy-final.log |
| Full tests of all seven touched crates | PASS | /tmp/drb-delivery-tests.log |
| Full Store tests after the final private gift-code correction | PASS | /tmp/drb-store-delivery.log |
| Existing ignored Store Chromium test, explicitly forced with --ignored | PASS, 360px PL/EN, successful transfer/refusal, axe 0 violations | /tmp/drb-browser-store.log |
| Existing ignored checkout Chromium test, explicitly forced with --ignored | PASS, dummy bank → actual worker credit → transfer, axe 0 violations | /tmp/drb-browser-checkout.log |
| Bootstrap CLI permission-revocation serialization | PASS | /tmp/drb-bootstrap-cli-final.log |
| All seven catalog CLI tests | PASS | /tmp/drb-catalog-cli-final.log |
| Changed-file/diff inspection, git diff --check, task checkpoint validation | PASS | local review and tools/agents/checkpoint.py |

Full command:

```text
cargo test --no-fail-fast -p oteryn-admin -p oteryn-bazaar -p oteryn-characters -p oteryn-content -p oteryn-db -p oteryn-store -p oteryn-support -- --test-threads=1
```

The final private Store correction was followed by the full Store suite and
workspace Clippy/formatting. Other touched-crate source was unchanged. Existing
ignored browser cases were executed explicitly rather than removed or weakened.

| Crate | Passed | Failed | Unexecuted |
|---|---:|---:|---:|
| oteryn-admin | 171 | 0 | 0 |
| oteryn-bazaar | 131 | 0 | 0 |
| oteryn-characters | 93 | 0 | 0 |
| oteryn-content | 31 | 0 | 0 |
| oteryn-db | 8 | 0 | 0 |
| oteryn-store | 120 | 0 | 0 |
| oteryn-support | 94 | 0 | 0 |
| Total touched-crate tests | 648 | 0 | 0 |

Store consists of 119 ordinary tests plus its explicitly executed browser test.
Nested filtered subprocess results are not counted twice. An additional nine
portal CLI/browser tests passed (bootstrap 1, catalog 7, checkout browser 1), for
657 distinct passing tests across the reported final validation scope.

| New PostgreSQL regression | Red | Green |
|---|---|---|
| creation_waits_for_moderation_before_locking_its_account | 55P03 on staff account acquisition while creation retained it behind gate | PASS |
| bid_prepares_previous_leader_before_actor_account | 55P03 on ordered role writer's later account | PASS |
| browser_redemption_locks_code_before_account_and_rechecks_status | 55P03: browser retained account behind code row | PASS, other account writer succeeds; suspended account cannot redeem |
| editorial_writers_reject_grant_expired_during_resource_wait | committed announcement using an expired real durable MFA grant | PASS for announcement edit, event edit and publication; original versions retained |
| issuer_authority_wait_uses_shared_mutation_deadline | outer bound elapsed, issuance remained blocked | PASS, bounded refusal and zero issued intents |

All five regressions use pg_blocking_pids bound to held backend PIDs, without
sleeps. Grant expiry uses PostgreSQL clock_timestamp during the retained wait.
The shared mutation acquisition deadline remains exactly three seconds. No new
allow/ignore attributes, timeout increases, mutation retries or skipped tests.

Delivery: exactly one requested commit, author blakinio
<75369544+blakinio@users.noreply.github.com>, on the allocated branch. The branch is
retained for owner/coordinator review; no PR creation, merge or deployment.
