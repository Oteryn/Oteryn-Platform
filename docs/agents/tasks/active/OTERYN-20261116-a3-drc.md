---
task_id: OTERYN-20261116-a3-drc
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Exhaustive lock-site inventory and remaining post-wait checks

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN,
read through GitHub during this task). Owner-authorized base
`f74eb95f91d28feefba8d02278ff5db1c4afe61d`, dedicated single-writer branch
`claude/happy-davinci-ijz3wy-fix-a3-drc`. Publication is limited to that branch.
No merge, deployment, live-data or Game repository operation is authorized.
Open Platform PRs were checked for overlapping audit ownership; predecessor audit
packets are historical context. Jira synchronization remains with the coordinator.

## Goal and acceptance

Repair the supplied post-wait authorization, acquisition-budget and account /
character-issuance inversion findings; enumerate all matching sites, including
SQL trigger/function provenance, fixture barriers and permission-only references.
The normative hierarchy is exclusively
`portal-rs/crates/admin/src/rbac/README.md`.

- [x] Seven PostgreSQL regressions reproduced the defects before repairs.
- [x] Twelve focused regressions pass after repairs on PostgreSQL 18.6.
- [x] Final fmt, workspace Clippy and all required crate/bin suites pass.
- [x] Candidate validated and ready for the requested commit and branch publication.

## Ownership

```yaml
owned_paths:
  - portal-rs/
  - docs/agents/tasks/active/OTERYN-20261116-a3-drc.md
modules:
  - transactional authorization and PostgreSQL lock protocol
dependencies: []
blockers:
  - none
cross_repository_tasks:
  - none
```

## Evidence and transaction review

Local evidence is under `/tmp/drc-evidence/`. The initial repository HEAD printed
before checkout was `90234d8d2d0a51a654b8ff6947d38372a740acba`.
The six supplied-path regressions failed against the requested base: kick accepted
an expired grant after a proven counter wait; Bazaar, gift redemption and catalog
audit exceeded their acquisition bound; admission retained the issuance fence
while waiting for the account; download authorization bypassed the RBAC writer.
A seventh red test found the shared helper increased an existing 100 ms lock
limit. It now retains the original deadline and any shorter caller limits.

Account-before-issuance applies to admission, session admission, login reads and
item reads, and to character service submission. Settlement prepares its account
before restore and command rows. Bazaar entries establish the budget at their
shared gate. Direct advisory helpers refresh the remaining budget; transaction
entries establish it before earlier row locks. Email claims and loyalty paths
include indirect advisory acquisition through their helpers / SQL functions.
Nonblocking analytics readers and the session leadership acquisition are included.
The session leadership lease protects worker ownership only; each domain mutation
still uses its own transaction and budget. Cancellation closes the lease connection.

Browser session rotation prepares both old and new accounts before the session
row; identity entries cover indirect signup-trigger advisory acquisition.
Kick, logout, reset, console enqueue and sanction export effects revalidate current
actor authority after their adapter/trigger effects. Download publication prepares
account, channel and RBAC through the canonical resource helper and checks current
permission on the publication connection through final commit preparation.
The only migration change is additive `20261116041001_a3_drc_lock_budgets.sql`.
Historical advisory trigger bodies are superseded there; no applied migration changed.

## Inventory interpretation

One row below corresponds to one matching source line, including permission enum
values, pure reads, adapter calls, comments, tests and historical SQL bodies.
Repeated matching tokens on a line are one site. This intentionally over-includes
`_in(` calls so permission reads hidden behind adapters cannot be omitted.
`pg_try_advisory*` is included in addition to literal `pg_advisory*`.

Modes/resources describe direct SQL at the site; permission/call rows inherit their
owning transaction's protocol. The hierarchy level refers to the README, with
independent-domain order specified in its advisory inventory. Budget `yes` includes
entry/canonical-helper establishment before a retained callback/resource lock.
Budget `n.a.` covers non-executable references, fixture barriers, migration
installation/provenance, and independent row-only paths that join neither advisory
nor RBAC mutation acquisition. Authorization `n.a.` covers those sites and
non-privileged internal/owner/read operations. Pure reads without locks are n.a.
Denied-access audit writes do not require a successful privileged grant.
`fixed in this commit` means the owning file's transaction protocol was repaired;
the matching SQL text need not itself have changed.

Domain pair review uses complete resource sets, rather than numeric advisory-key
order: Bazaar shared/exclusive gate before listings/accounts; gift request → code
→ batch/account → account; accounts → character issuance → restore → command and
projection rows; ordered actor/subject accounts → prepared target → RBAC → durable
session/MFA → adapter effects; control ownership → ingress → request; player-data
account serialization → authority → stream rows; restore gates before receipt rows;
store restore before delivery issuer and operation rows; referral ordered account /
authority preparation before graph serialization. Single-domain request keys do
not cross domains. New private rows do not constitute reverse existing-row waits.

Regenerate the match set with:

```sh
cd portal-rs
rg -n -i 'pg_(try_)?advisory|FOR (NO KEY UPDATE|KEY SHARE|UPDATE|SHARE)|LOCK TABLE|Permission::|authorize|role_permissions|account_roles|\w+_in\(' crates bin migrations
```

Inventory: **2670 matching lines**; documentation/data: 36, migration: 92, production: 1647, test fixture/barrier: 895.

| File:line | Locks / mode / site context | Hierarchy level | Budget before first lock | Authorization after last wait | Compliant |
|---|---|---|---|---|---|
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:534` | no direct lock; permission/admission/call site (test fixture/barrier; production_suspended_report_notification) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:75` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:401` | no direct lock; permission/admission/call site (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:403` | no direct lock; permission/admission/call site (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:425` | no direct lock; permission/admission/call site (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:435` | no direct lock; permission/admission/call site (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:449` | FOR UPDATE: character_bootstrap_intent_authority (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:474` | no direct lock; permission/admission/call site (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:478` | no direct lock; permission/admission/call site (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:493` | pg_advisory_xact_lock: pg_stat_activity, 71010 (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:504` | no direct lock; permission/admission/call site (test fixture/barrier; cw3_02_postgres_bootstrap_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:21` | FOR SHARE (production; account_state) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:25` | FOR SHARE: accounts (production; account_state) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:53` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; permitted) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:54` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; permitted) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:56` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; permitted) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:58` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; permitted) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:118` | pg_advisory_xact_lock: 281510004 (production; audit) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:47` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:48` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:50` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:56` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:194` | no direct lock; permission/admission/call site (test fixture/barrier; catalog_cli_refuses_actor_without_permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:276` | pg_try_advisory_xact_lock: 71010 (test fixture/barrier; delta_catalog_permission_read_retains_rbac_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:329` | no direct lock; permission/admission/call site (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:331` | no direct lock; permission/admission/call site (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:353` | no direct lock; permission/admission/call site (test fixture/barrier; delta_catalog_activation_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:363` | no direct lock; permission/admission/call site (test fixture/barrier; delta_catalog_activation_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:373` | FOR UPDATE: store_active_catalog (test fixture/barrier; delta_catalog_activation_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:396` | no direct lock; permission/admission/call site (test fixture/barrier; delta_catalog_activation_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:405` | no direct lock; permission/admission/call site (test fixture/barrier; delta_catalog_activation_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:413` | pg_advisory_xact_lock: pg_stat_activity, 71010 (test fixture/barrier; delta_catalog_activation_serializes_permission_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:442` | pg_advisory_xact_lock: 281510004 (test fixture/barrier; postgres_drc_catalog_audit_has_shared_budget) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:32` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:112` | no direct lock; permission/admission/call site (production; configured_from) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:145` | no direct lock; permission/admission/call site (production; budget) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:176` | no direct lock; permission/admission/call site (production; budget) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:181` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:184` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:243` | no direct lock; permission/admission/call site (production; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:52` | no direct lock; permission/admission/call site (test fixture/barrier; db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:100` | no direct lock; permission/admission/call site (test fixture/barrier; db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:188` | no direct lock; permission/admission/call site (test fixture/barrier; db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:52` | no direct lock; permission/admission/call site (test fixture/barrier; audit_09_01_real_wallet_serializes_ceiling_and_holds_paid_settlement) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:90` | no direct lock; permission/admission/call site (test fixture/barrier; audit_09_01_real_wallet_serializes_ceiling_and_holds_paid_settlement) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:96` | no direct lock; permission/admission/call site (test fixture/barrier; audit_09_01_real_wallet_serializes_ceiling_and_holds_paid_settlement) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:120` | no direct lock; permission/admission/call site (test fixture/barrier; audit_09_01_real_wallet_serializes_ceiling_and_holds_paid_settlement) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:92` | no direct lock; permission/admission/call site (production; payer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:135` | FOR SHARE: accounts (production; restriction_in) | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:140` | FOR SHARE: accounts (production; restriction_in) | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:164` | FOR UPDATE: wallet_accounts (production; credit_in) | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:175` | no direct lock; permission/admission/call site (production; credit_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:221` | no direct lock; permission/admission/call site (production; reverse_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:248` | no direct lock; permission/admission/call site (production; wallet) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:284` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:67` | no direct lock; permission/admission/call site (test fixture/barrier; restriction_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:88` | no direct lock; permission/admission/call site (test fixture/barrier; reverse_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:97` | no direct lock; permission/admission/call site (test fixture/barrier; credit_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:180` | no direct lock; permission/admission/call site (test fixture/barrier; tokens) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:502` | no direct lock; permission/admission/call site (test fixture/barrier; catalogue_order_paid_credits_once) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:605` | no direct lock; permission/admission/call site (test fixture/barrier; scopes_are_separate_tokens) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:617` | no direct lock; permission/admission/call site (test fixture/barrier; scopes_are_separate_tokens) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:625` | no direct lock; permission/admission/call site (test fixture/barrier; scopes_are_separate_tokens) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:664` | no direct lock; permission/admission/call site (test fixture/barrier; scopes_are_separate_tokens) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:715` | no direct lock; permission/admission/call site (test fixture/barrier; refusals_change_nothing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:717` | no direct lock; permission/admission/call site (test fixture/barrier; refusals_change_nothing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1460` | no direct lock; permission/admission/call site (test fixture/barrier; reversal_of_unavailable_coins_holds_and_freezes_purchasing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1472` | no direct lock; permission/admission/call site (test fixture/barrier; reversal_of_unavailable_coins_holds_and_freezes_purchasing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1972` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1978` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2007` | no direct lock; permission/admission/call site (test fixture/barrier; l4_04_native_purchase_reaches_real_analytics_once) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2094` | no direct lock; permission/admission/call site (test fixture/barrier; l2_01_shared_transfer_ceiling_refuses_atomically_and_native_balance_reads) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2095` | no direct lock; permission/admission/call site (test fixture/barrier; l2_01_shared_transfer_ceiling_refuses_atomically_and_native_balance_reads) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2159` | FOR UPDATE: accounts (test fixture/barrier; settlement_and_store_read_sanctions_committed_during_account_lock_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_devices_sanctions_tests.rs:168` | no direct lock; permission/admission/call site (test fixture/barrier; sanctioned_device_is_refused) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:60` | no direct lock; permission/admission/call site (test fixture/barrier; assert_authentication) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:521` | no direct lock; permission/admission/call site (test fixture/barrier; pg_recovery_refusals_fail_closed_without_recording_an_attempt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:522` | no direct lock; permission/admission/call site (test fixture/barrier; pg_recovery_refusals_fail_closed_without_recording_an_attempt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:96` | no direct lock; permission/admission/call site (test fixture/barrier; access_token) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:364` | no direct lock; permission/admission/call site (test fixture/barrier; pg_oauth_token_ticket_and_admission_through_real_routers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:367` | no direct lock; permission/admission/call site (test fixture/barrier; pg_oauth_token_ticket_and_admission_through_real_routers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:377` | no direct lock; permission/admission/call site (test fixture/barrier; pg_oauth_token_ticket_and_admission_through_real_routers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:378` | no direct lock; permission/admission/call site (test fixture/barrier; pg_oauth_token_ticket_and_admission_through_real_routers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:420` | no direct lock; permission/admission/call site (test fixture/barrier; pg_oauth_token_ticket_and_admission_through_real_routers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:433` | no direct lock; permission/admission/call site (test fixture/barrier; pg_oauth_token_ticket_and_admission_through_real_routers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:456` | no direct lock; permission/admission/call site (test fixture/barrier; pg_suspended_account_gets_no_ticket_characters_or_admission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:462` | no direct lock; permission/admission/call site (test fixture/barrier; pg_suspended_account_gets_no_ticket_characters_or_admission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:475` | no direct lock; permission/admission/call site (test fixture/barrier; pg_suspended_account_gets_no_ticket_characters_or_admission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:502` | no direct lock; permission/admission/call site (test fixture/barrier; pg_lcfa_internal_identity_is_exclusive) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:884` | no direct lock; permission/admission/call site (test fixture/barrier; pg_testing_issuance_signs_a_grant_the_trust_registry_verifies) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1041` | no direct lock; permission/admission/call site (test fixture/barrier; pg_issuance_refusals_fail_closed_without_consuming_the_ticket) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1132` | no direct lock; permission/admission/call site (test fixture/barrier; pg_signer_self_check_failures_refuse_issuance_without_consuming_the_ticket) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1409` | FOR UPDATE: game_channels (test fixture/barrier; pg_route_selection_filters_the_channel_first_and_locks_only_its_record) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes.rs:529` | no direct lock; permission/admission/call site (production; watermark) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:135` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:362` | no direct lock; permission/admission/call site (test fixture/barrier; db_mounted_purchase_accepts_once_replays_bytes_and_refuses_conflicts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:366` | no direct lock; permission/admission/call site (test fixture/barrier; db_mounted_purchase_accepts_once_replays_bytes_and_refuses_conflicts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:995` | no direct lock; permission/admission/call site (test fixture/barrier; db_g9_revision3_goldens_through_binary_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:348` | no direct lock; permission/admission/call site (test fixture/barrier; each_route_answers_its_identity_refuses_others_and_is_off_without_its_flag) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:408` | no direct lock; permission/admission/call site (test fixture/barrier; character_command_identities_reach_only_the_command_routes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:420` | no direct lock; permission/admission/call site (test fixture/barrier; character_command_identities_reach_only_the_command_routes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:497` | no direct lock; permission/admission/call site (test fixture/barrier; v2_watermark_reaches_stats_only_for_its_families) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:529` | no direct lock; permission/admission/call site (test fixture/barrier; g4_character_families_bind_the_public_identity_and_flags) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:534` | no direct lock; permission/admission/call site (test fixture/barrier; g4_character_families_bind_the_public_identity_and_flags) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:545` | no direct lock; permission/admission/call site (test fixture/barrier; g4_character_families_bind_the_public_identity_and_flags) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:627` | no direct lock; permission/admission/call site (test fixture/barrier; journal_ingest_binds_its_purpose_source_and_environment) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:632` | no direct lock; permission/admission/call site (test fixture/barrier; journal_ingest_binds_its_purpose_source_and_environment) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:703` | no direct lock; permission/admission/call site (test fixture/barrier; store_purchases_stay_off_outside_testing_and_preproduction) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:753` | no direct lock; permission/admission/call site (test fixture/barrier; preprod_reaches_every_enabled_environment_gated_surface) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:805` | no direct lock; permission/admission/call site (test fixture/barrier; enforcement_routes_bind_their_two_identities_behind_their_flag) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:815` | no direct lock; permission/admission/call site (test fixture/barrier; enforcement_routes_bind_their_two_identities_behind_their_flag) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:965` | no direct lock; permission/admission/call site (test fixture/barrier; watermark_dispatch_refuses_before_polling_disabled_or_wrong_peer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/internal_tls_tests.rs:304` | no direct lock; permission/admission/call site (test fixture/barrier; public_listener_never_verifies_a_client) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:66` | no direct lock; permission/admission/call site (test fixture/barrier; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:82` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:88` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:126` | no direct lock; permission/admission/call site (production; run) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:131` | no direct lock; permission/admission/call site (production; run) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:270` | FOR UPDATE: accounts (production; account) | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:391` | FOR UPDATE: game_channels (production; channel_record) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:420` | no direct lock; permission/admission/call site (test fixture/barrier; bootstrap_intent_is_issued_once_and_readable) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:476` | no direct lock; permission/admission/call site (test fixture/barrier; every_seed_action_refuses_outside_testing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:809` | no direct lock; permission/admission/call site (test fixture/barrier; signing_custody_runs_only_in_testing_or_preprod) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:812` | no direct lock; permission/admission/call site (test fixture/barrier; signing_custody_runs_only_in_testing_or_preprod) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:106` | FOR UPDATE: payment_intents (production; credit) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:113` | FOR SHARE: store_web_topups (production; credit) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:144` | no direct lock; permission/admission/call site (production; credit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:164` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:172` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_binary_test.py:54` | FOR UPDATE: payment_intents (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_binary_test.py:62` | FOR UPDATE: pg_stat_activity, pg_database, payment_intents (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:344` | no direct lock; permission/admission/call site (test fixture/barrier; paid_topup_and_refund_scenario) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:486` | no direct lock; permission/admission/call site (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:491` | no direct lock; permission/admission/call site (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:707` | FOR UPDATE (test fixture/barrier; topup_refusals_are_audited_and_dead_lettered_without_credit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:737` | FOR UPDATE: payment_intents (test fixture/barrier; locked_topup) | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:1256` | LOCK TABLE: outbox (test fixture/barrier; claiming_table_lock_has_a_deadline_and_closes_blocked_session) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/README.md:107` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/economy.rs:58` | pg_try_advisory_xact_lock: 71140001 (production; read) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/economy.rs:156` | no direct lock; permission/admission/call site (production; read) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/mod.rs:163` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/purchased.rs:19` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/store.rs:85` | pg_advisory_xact_lock: 71140001 (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/store.rs:88` | FOR UPDATE: admin_analytics.fence (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/store.rs:134` | FOR UPDATE: admin_analytics.watermarks (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/store.rs:313` | FOR UPDATE: admin_analytics.subjects (production; commit) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/store.rs:405` | pg_advisory_xact_lock: 71140001 (production; purge_bounded) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/tests.rs:103` | no direct lock; permission/admission/call site (test fixture/barrier; wrong_purpose_transport_and_bounds) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:306` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:312` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:356` | no direct lock; permission/admission/call site (test fixture/barrier; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:466` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:478` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:484` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:491` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:497` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:505` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:512` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:525` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_audit_before_read_outage_and_rbac_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:856` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_reversible_analytics_migration_preserves_role_catalog) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:880` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_reversible_analytics_migration_preserves_role_catalog) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1497` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_platform_purchased_units_empty_day_and_boundaries) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1508` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_platform_purchased_units_empty_day_and_boundaries) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1514` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_platform_purchased_units_empty_day_and_boundaries) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1571` | FOR UPDATE: admin_analytics.pages (test fixture/barrier; a2_blocked_purge_refuses_browser_reads_without_pool_exhaustion) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:7` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:37` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:108` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:125` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:192` | no direct lock; permission/admission/call site (production; denial) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:213` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:215` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/analytics/web.rs:260` | pg_try_advisory_xact_lock: 71140001 (production; read_bounded) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/audit_control.rs:1` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:38` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:44` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:56` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:58` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:62` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:73` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:75` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:93` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; execute) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:98` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; execute) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:101` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; execute) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:104` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; execute) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:120` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; execute) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:144` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar.rs:163` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/bazaar_tests.rs:14` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:20` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:81` | no direct lock; permission/admission/call site (test fixture/barrier; with_connections) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:182` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_authorization_mfa_and_audit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:335` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_four_eyes_requires_second_live_permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:497` | FOR UPDATE: bazaar_listings (test fixture/barrier; postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:504` | FOR UPDATE: bazaar_reports (test fixture/barrier; postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:511` | pg_advisory_xact_lock: 64001 (test fixture/barrier; postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:517` | pg_advisory_xact_lock: 64002 (test fixture/barrier; postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:593` | pg_advisory_xact_lock_shared: 64002 (test fixture/barrier; delta_bazaar_listing_before_staff_account_avoids_deadlock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:601` | FOR UPDATE: bazaar_listings (test fixture/barrier; delta_bazaar_listing_before_staff_account_avoids_deadlock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:650` | no direct lock; permission/admission/call site (test fixture/barrier; delta_bazaar_listing_before_staff_account_avoids_deadlock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:12` | FOR SHARE: accounts (production; state) | 3 | yes | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:22` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:31` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; adjustment) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:45` | no direct lock; permission/admission/call site (production; inspect) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:79` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:84` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:109` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:161` | FOR UPDATE: commerce_refunds, commerce_operations (production; deliver_refund) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:174` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; deliver_refund) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:226` | no direct lock; permission/admission/call site (production; inspect) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:246` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; apply) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:318` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:324` | FOR SHARE: accounts (production; state_in) | 3 | yes | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:46` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:76` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:188` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:48` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:49` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:50` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:141` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:148` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:154` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:156` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:158` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:172` | no direct lock; permission/admission/call site (production; authorize_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:173` | no direct lock; permission/admission/call site (production; authorize_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:198` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:203` | FOR UPDATE: commerce_operations (production; submit) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:212` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:243` | FOR UPDATE: commerce_operations (production; submit) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:253` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:263` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:290` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:321` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:326` | FOR UPDATE: commerce_operations (production; approve) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:335` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:338` | FOR UPDATE: commerce_operations (production; approve) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:344` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:361` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:363` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:384` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:17` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:23` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:29` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:34` | no direct lock; permission/admission/call site (test fixture/barrier; state_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:58` | FOR SHARE: accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:70` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:77` | no direct lock; permission/admission/call site (test fixture/barrier; adjustment) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:186` | no direct lock; permission/admission/call site (test fixture/barrier; with_connections) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:361` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_wallet_four_eyes_atomicity_concurrency_and_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:712` | no direct lock; permission/admission/call site (test fixture/barrier; a2_configured_commerce_domain_adapters_commit_ledgers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:845` | pg_advisory_xact_lock: 71010 (test fixture/barrier; a2_commerce_role_writer_waits_are_bounded) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:145` | no direct lock; permission/admission/call site (production; approvals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:159` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:407` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:410` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:411` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:414` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:581` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:25` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:31` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:196` | no direct lock; permission/admission/call site (test fixture/barrier; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:201` | no direct lock; permission/admission/call site (test fixture/barrier; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:147` | no direct lock; permission/admission/call site (production; navigation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:162` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:166` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:349` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:398` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:332` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:338` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:365` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_four_pages_filters_host_and_read_only) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:488` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_four_pages_filters_host_and_read_only) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:150` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:158` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:164` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:182` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:185` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:193` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:201` | FOR SHARE: account_mfa_tokens, web_sessions (production; submit_query) | 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:204` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:250` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:296` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored.rs:299` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; submit_query) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/console/stored_tests.rs:227` | no direct lock; permission/admission/call site (test fixture/barrier; a2_stored_console_browser_query_verified_result_and_ownership) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:228` | no direct lock; permission/admission/call site (test fixture/barrier; a2_stored_console_browser_query_verified_result_and_ownership) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:276` | no direct lock; permission/admission/call site (test fixture/barrier; a2_stored_console_browser_query_verified_result_and_ownership) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:91` | no direct lock; permission/admission/call site (production; policy) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:131` | no direct lock; permission/admission/call site (production; present_string) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:107` | no direct lock; permission/admission/call site (test fixture/barrier; permission_risk_table_and_untrusted_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:108` | no direct lock; permission/admission/call site (test fixture/barrier; permission_risk_table_and_untrusted_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:109` | no direct lock; permission/admission/call site (test fixture/barrier; permission_risk_table_and_untrusted_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:110` | no direct lock; permission/admission/call site (test fixture/barrier; permission_risk_table_and_untrusted_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:111` | no direct lock; permission/admission/call site (test fixture/barrier; permission_risk_table_and_untrusted_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:112` | no direct lock; permission/admission/call site (test fixture/barrier; permission_risk_table_and_untrusted_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:84` | no direct lock; permission/admission/call site (production; recent) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:153` | no direct lock; permission/admission/call site (production; verify_totp) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:354` | no direct lock; permission/admission/call site (production; enabled) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:361` | no direct lock; permission/admission/call site (production; authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:363` | no direct lock; permission/admission/call site (production; authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:370` | no direct lock; permission/admission/call site (production; authorized_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:381` | no direct lock; permission/admission/call site (production; now) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:383` | no direct lock; permission/admission/call site (production; now) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:448` | no direct lock; permission/admission/call site (production; preview) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:450` | no direct lock; permission/admission/call site (production; preview) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:37` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:67` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:204` | no direct lock; permission/admission/call site (production; approval_page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:302` | no direct lock; permission/admission/call site (production; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:313` | no direct lock; permission/admission/call site (production; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:319` | no direct lock; permission/admission/call site (production; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:71` | no direct lock; permission/admission/call site (test fixture/barrier; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:445` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:451` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:467` | no direct lock; permission/admission/call site (test fixture/barrier; pg_forms_csrf_locales_and_step_up_redirect) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:680` | no direct lock; permission/admission/call site (test fixture/barrier; pg_approval_form_review_csrf_and_submit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:808` | no direct lock; permission/admission/call site (test fixture/barrier; record_preview) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:852` | no direct lock; permission/admission/call site (test fixture/barrier; approve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:902` | no direct lock; permission/admission/call site (test fixture/barrier; evidence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:237` | no direct lock; permission/admission/call site (production; dispatch_peer) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/http.rs:300` | no direct lock; permission/admission/call site (production; hello) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/http.rs:417` | no direct lock; permission/admission/call site (production; results) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/http.rs:436` | pg_advisory_xact_lock: 17 (production; lock_request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/http.rs:653` | pg_advisory_xact_lock_shared: 0, admin-control-http (production; ingress_lock) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/http.rs:659` | pg_advisory_xact_lock: 0, admin-control-http (production; ingress_lock) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/http_tests.rs:486` | LOCK TABLE: admin_control_agents (test fixture/barrier; postgres_hello_uses_one_connection_under_ingress_contention) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:506` | pg_advisory_xact_lock_shared: admin-control-http (test fixture/barrier; postgres_hello_uses_one_connection_under_ingress_contention) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:586` | LOCK TABLE: admin_control_results (test fixture/barrier; postgres_result_uses_one_connection_under_stream_contention) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:599` | pg_advisory_xact_lock: 17 (test fixture/barrier; postgres_result_uses_one_connection_under_stream_contention) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:33` | no direct lock; permission/admission/call site (production; control_evidence) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:75` | no direct lock; permission/admission/call site (production; bound_step) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:81` | no direct lock; permission/admission/call site (production; bound_step) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:82` | no direct lock; permission/admission/call site (production; bound_step) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:105` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; record_request_preview) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:124` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; record_request_preview) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:151` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; record_request_preview) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:169` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; approve_request) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:171` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; approve_request) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:188` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; approve_request) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:194` | request and preview UPDATE in UUID order → ordered accounts → RBAC → session/MFA (owning workflow) (production; approve_request) | 2 → 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:212` | no direct lock; permission/admission/call site (production; control_evidence) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:228` | no direct lock; permission/admission/call site (production; control_evidence) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:238` | no direct lock; permission/admission/call site (production; control_evidence_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:265` | no direct lock; permission/admission/call site (production; control_evidence_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:324` | no direct lock; permission/admission/call site (production; control_evidence_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:331` | no direct lock; permission/admission/call site (production; control_evidence_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration.rs:348` | FOR UPDATE: admin_control_requests (production; prepare_control_requests) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/integration_tests.rs:71` | no direct lock; permission/admission/call site (test fixture/barrier; enqueue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration_tests.rs:305` | no direct lock; permission/admission/call site (test fixture/barrier; stored_approval_refuses_self_expired_and_rolls_back_with_audit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:286` | no direct lock; permission/admission/call site (production; register_agent) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:291` | no direct lock; permission/admission/call site (production; register_agent) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:345` | no direct lock; permission/admission/call site (production; enqueue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:349` | no direct lock; permission/admission/call site (production; enqueue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:417` | FOR UPDATE: admin_control_requests (production; lock_control_request) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:519` | FOR SHARE (production; claim_pending) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:544` | FOR UPDATE (production; claim_pending) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:594` | no direct lock; permission/admission/call site (production; record_result) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:599` | no direct lock; permission/admission/call site (production; record_result) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:608` | FOR UPDATE: admin_control_requests (production; record_result_in) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:683` | FOR UPDATE SKIP LOCKED (production; expire_due) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:851` | pg_advisory_xact_lock_shared: 0, admin-control-ownership (production; ownership_lock) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:857` | pg_advisory_xact_lock: 0, admin-control-ownership (production; ownership_lock) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/store.rs:867` | no direct lock; permission/admission/call site (production; ownership_lock) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/control/tests.rs:397` | FOR UPDATE: admin_control_requests (test fixture/barrier; pg_expiry_is_terminal_and_does_not_expire_accepted_work) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/tests.rs:448` | FOR UPDATE: admin_control_requests (test fixture/barrier; pg_claim_skips_locked_rows_and_parallel_batches_are_disjoint) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/tests.rs:755` | FOR UPDATE: admin_control_agents (test fixture/barrier; pg_claim_checks_live_expiry_after_waiting_for_an_agent_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/lib.rs:49` | no direct lock; permission/admission/call site (production; test_staff_authenticate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:23` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:44` | no direct lock; permission/admission/call site (production; configured_router_from) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:66` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:236` | no direct lock; permission/admission/call site (production; search) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:5` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:173` | no direct lock; permission/admission/call site (production; reset_mfa) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:179` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:193` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:197` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:210` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:212` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:226` | no direct lock; permission/admission/call site (production; search) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:239` | no direct lock; permission/admission/call site (production; account) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:246` | no direct lock; permission/admission/call site (production; sessions) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:261` | no direct lock; permission/admission/call site (production; characters) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:275` | no direct lock; permission/admission/call site (production; notes) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:283` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; note) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:288` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; note) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:293` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; note) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:296` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; note) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:314` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; note) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:325` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; logout_all) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:330` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; logout_all) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:334` | FOR UPDATE: web_sessions (production; logout_all) | 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:340` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; logout_all) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:343` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; logout_all) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:346` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; logout_all) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:356` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; logout_all) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:366` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; kick) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:370` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; kick) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:375` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; kick) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:378` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; kick) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:390` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; kick) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:400` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; kick) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:411` | no direct lock; permission/admission/call site (production; coin_hold) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:422` | no direct lock; permission/admission/call site (production; coin_debt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:443` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; release_coin_hold) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:452` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; release_coin_hold) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:457` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; release_coin_hold) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:460` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; release_coin_hold) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:473` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; release_coin_hold) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:492` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; release_coin_hold) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:522` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; attest_reset_evidence_impl) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:532` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; attest_reset_evidence_impl) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:537` | FOR UPDATE: tickets (production; attest_reset_evidence_impl) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:546` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; attest_reset_evidence_impl) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:549` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; attest_reset_evidence_impl) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:563` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; attest_reset_evidence_impl) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:575` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; request_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:584` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; request_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:589` | FOR UPDATE: support_mfa_reset_evidence (production; request_reset) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:598` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; request_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:601` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; request_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:606` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; request_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:617` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; request_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:628` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:641` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:645` | FOR UPDATE: support_mfa_reset_evidence, player_mfa_reset_requests (production; approve_reset) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:646` | FOR UPDATE: web_sessions (production; approve_reset) | 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:651` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:654` | FOR UPDATE: player_mfa_reset_requests (production; approve_reset) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:659` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:661` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:663` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:669` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:683` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/mod.rs:700` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; approve_reset) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/players/tests.rs:19` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:25` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:156` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:311` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_reset_refuses_missing_evidence_expiry_and_revoked_requester) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:605` | no direct lock; permission/admission/call site (test fixture/barrier; kick_fixture) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1292` | pg_advisory_xact_lock: 71010 (test fixture/barrier; postgres_reset_rechecks_live_expiry_after_advisory_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1841` | FOR UPDATE: identity_security_revocation_counter (test fixture/barrier; postgres_drc_kick_rechecks_grant_after_revocation_counter_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1901` | FOR UPDATE: test_security (test fixture/barrier; postgres_drc_reset_request_rechecks_expiry_after_adapter_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1968` | LOCK TABLE: audit_log (test fixture/barrier; postgres_drc_kick_rechecks_grant_after_audit_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:19` | FOR SHARE, FOR UPDATE: staff (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:21` | FOR UPDATE, FOR NO KEY UPDATE, FOR SHARE, FOR KEY SHARE (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:22` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:70` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:72` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:5` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:314` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:315` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:316` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:317` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:318` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:319` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:320` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:321` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:322` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:323` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:324` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:325` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:326` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:327` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:328` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:329` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:330` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:331` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:332` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:333` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:334` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:335` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:336` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:337` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:338` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:339` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:340` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:341` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:342` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:343` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:344` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:345` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:346` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:347` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:348` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:349` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:350` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:353` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:354` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:355` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:358` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:359` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:360` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:361` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:362` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:363` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:364` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:365` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:366` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:367` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:368` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:369` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:370` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:373` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:374` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:375` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:378` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:379` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:380` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:381` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:384` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:385` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:386` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:387` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:388` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:389` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:390` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:391` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:392` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:393` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:394` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:397` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:398` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:399` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:400` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:401` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:402` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:403` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:404` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:405` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:408` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:409` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:410` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:411` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:412` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:413` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:414` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:415` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:416` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:417` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:418` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:419` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:422` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:423` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:424` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:425` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:426` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:427` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:428` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:429` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:430` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:431` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:432` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:433` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:434` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:435` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:436` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:437` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:438` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:441` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:442` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:443` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:444` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:445` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:446` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:447` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:448` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:449` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:450` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:451` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:452` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:453` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:454` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:457` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:458` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:459` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:460` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:461` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:462` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:463` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:464` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:465` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:466` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:467` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:468` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:469` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:470` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:471` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:472` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:473` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:474` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:477` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:478` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:479` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:480` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:483` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:484` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:485` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:486` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:489` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:490` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:491` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:492` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:493` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:494` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:495` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:497` | no direct lock; permission/admission/call site (production; permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:33` | no direct lock; permission/admission/call site (production; context) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:42` | no direct lock; permission/admission/call site (production; from_request_parts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:43` | no direct lock; permission/admission/call site (production; from_request_parts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:44` | no direct lock; permission/admission/call site (production; from_request_parts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:67` | FOR UPDATE: accounts (production; lock_accounts_resources_and_mutation_with) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:73` | pg_advisory_xact_lock: 71010 (production; lock_accounts_resources_and_mutation_with) | 5 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:133` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:140` | no direct lock; permission/admission/call site (production; authenticated_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:148` | no direct lock; permission/admission/call site (production; authenticated_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:159` | no direct lock; permission/admission/call site (production; state_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:174` | FOR UPDATE: web_sessions, account_mfa_totp, account_mfa_tokens (production; durable_session_in) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:177` | FOR UPDATE: web_sessions, accounts, account_mfa_totp, account_mfa_tokens (production; durable_session_in) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:243` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:249` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:281` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:293` | no direct lock; permission/admission/call site (production; account_active) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:300` | no direct lock; permission/admission/call site (production; account_active_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:307` | no direct lock; permission/admission/call site (production; account_active_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:315` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:319` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:326` | no direct lock; permission/admission/call site (production; check_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:334` | no direct lock; permission/admission/call site (production; revalidate_actor_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:338` | no direct lock; permission/admission/call site (production; revalidate_actor_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:349` | no direct lock; permission/admission/call site (production; revalidate_actor_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:355` | no direct lock; permission/admission/call site (production; check_subject_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:368` | no direct lock; permission/admission/call site (production; check_permission) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:369` | no direct lock; permission/admission/call site (production; check_permission) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:407` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:414` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:418` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:424` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; new) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:430` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; check_in) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:435` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; check_in) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:453` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; check) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:460` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_mutation) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:472` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_mutation) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:486` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_mutation_with_resources) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/mod.rs:498` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_mutation_with_resources) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:1` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:46` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:158` | no direct lock; permission/admission/call site (test fixture/barrier; panel_refuses_last_eligible_administrator_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:165` | no direct lock; permission/admission/call site (test fixture/barrier; panel_refuses_last_eligible_administrator_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:182` | no direct lock; permission/admission/call site (test fixture/barrier; panel_refuses_last_eligible_administrator_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:194` | no direct lock; permission/admission/call site (test fixture/barrier; panel_refuses_last_eligible_administrator_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:212` | no direct lock; permission/admission/call site (test fixture/barrier; panel_refuses_last_eligible_administrator_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:279` | no direct lock; permission/admission/call site (test fixture/barrier; panel_creates_and_edits_roles_with_csrf_and_privilege_guards) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:281` | no direct lock; permission/admission/call site (test fixture/barrier; panel_creates_and_edits_roles_with_csrf_and_privilege_guards) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:291` | no direct lock; permission/admission/call site (test fixture/barrier; panel_creates_and_edits_roles_with_csrf_and_privilege_guards) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:297` | no direct lock; permission/admission/call site (test fixture/barrier; panel_creates_and_edits_roles_with_csrf_and_privilege_guards) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:342` | no direct lock; permission/admission/call site (test fixture/barrier; panel_creates_and_edits_roles_with_csrf_and_privilege_guards) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:375` | pg_advisory_xact_lock: 71010 (test fixture/barrier; player_notes_complete_when_small_pool_is_queued_on_role_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:422` | no direct lock; permission/admission/call site (test fixture/barrier; role_fence_wait_is_bounded_and_releases_pool_capacity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:426` | pg_advisory_xact_lock: 71010 (test fixture/barrier; role_fence_wait_is_bounded_and_releases_pool_capacity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:489` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_role_pages_and_survivor_batches_are_bounded) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:613` | FOR UPDATE: accounts (test fixture/barrier; postgres_a3_real_player_request_refuses_revoked_session_after_authentication) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:640` | pg_advisory_xact_lock, FOR UPDATE: pg_stat_activity, pg_database, accounts, 71010 (test fixture/barrier; staff_waiters) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:649` | pg_advisory_xact_lock: 71010 (test fixture/barrier; postgres_a3_staff_barrier_ignores_another_fixture_waiter) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:709` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_role_writer_rechecks_current_session_mfa_binding) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:711` | FOR UPDATE: accounts (test fixture/barrier; postgres_a3_role_writer_rechecks_current_session_mfa_binding) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:736` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_role_writer_rechecks_current_session_mfa_binding) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:1` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:28` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:68` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:73` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:76` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:79` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:83` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:95` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:131` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:138` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:164` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:171` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:174` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:212` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; change_impl) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:269` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; write_permissions) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/storage.rs:283` | no direct lock; permission/admission/call site (production; restrict_analytics) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/rbac/tests.rs:17` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:23` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:45` | no direct lock; permission/admission/call site (test fixture/barrier; permission_catalog_is_stable_unique_and_covered) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:48` | no direct lock; permission/admission/call site (test fixture/barrier; permission_catalog_is_stable_unique_and_covered) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:49` | no direct lock; permission/admission/call site (test fixture/barrier; permission_catalog_is_stable_unique_and_covered) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:57` | no direct lock; permission/admission/call site (test fixture/barrier; permission_catalog_is_stable_unique_and_covered) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:58` | no direct lock; permission/admission/call site (test fixture/barrier; permission_catalog_is_stable_unique_and_covered) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:59` | no direct lock; permission/admission/call site (test fixture/barrier; permission_catalog_is_stable_unique_and_covered) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:98` | no direct lock; permission/admission/call site (test fixture/barrier; account_and_mfa_fail_closed_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:100` | no direct lock; permission/admission/call site (test fixture/barrier; account_and_mfa_fail_closed_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:104` | no direct lock; permission/admission/call site (test fixture/barrier; account_and_mfa_fail_closed_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:106` | no direct lock; permission/admission/call site (test fixture/barrier; account_and_mfa_fail_closed_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:110` | no direct lock; permission/admission/call site (test fixture/barrier; account_and_mfa_fail_closed_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:117` | no direct lock; permission/admission/call site (test fixture/barrier; account_and_mfa_fail_closed_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:125` | no direct lock; permission/admission/call site (test fixture/barrier; guard_rejects_missing_or_unverified_identity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:130` | no direct lock; permission/admission/call site (test fixture/barrier; guard_rejects_missing_or_unverified_identity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:191` | no direct lock; permission/admission/call site (test fixture/barrier; close) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:192` | no direct lock; permission/admission/call site (test fixture/barrier; auth) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:206` | no direct lock; permission/admission/call site (test fixture/barrier; manager) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:232` | no direct lock; permission/admission/call site (test fixture/barrier; db_deny_by_default_and_refuse_unprivileged_mutation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:266` | no direct lock; permission/admission/call site (test fixture/barrier; db_grant_revoke_and_guard_use_current_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:287` | no direct lock; permission/admission/call site (test fixture/barrier; db_grant_revoke_and_guard_use_current_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:298` | no direct lock; permission/admission/call site (test fixture/barrier; db_grant_revoke_and_guard_use_current_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:324` | no direct lock; permission/admission/call site (test fixture/barrier; db_custom_role_permissions_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:337` | no direct lock; permission/admission/call site (test fixture/barrier; db_custom_role_permissions_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:347` | no direct lock; permission/admission/call site (test fixture/barrier; db_custom_role_permissions_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:351` | no direct lock; permission/admission/call site (test fixture/barrier; db_custom_role_permissions_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:365` | no direct lock; permission/admission/call site (test fixture/barrier; db_custom_role_permissions_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:433` | no direct lock; permission/admission/call site (test fixture/barrier; db_audit_change_commits_atomically_with_actor_and_diff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:461` | no direct lock; permission/admission/call site (test fixture/barrier; db_audit_failure_rolls_back_role_change) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:514` | no direct lock; permission/admission/call site (test fixture/barrier; db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:534` | no direct lock; permission/admission/call site (test fixture/barrier; db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:536` | no direct lock; permission/admission/call site (test fixture/barrier; db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:544` | no direct lock; permission/admission/call site (test fixture/barrier; db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:549` | no direct lock; permission/admission/call site (test fixture/barrier; db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:562` | no direct lock; permission/admission/call site (test fixture/barrier; db_built_in_seeds_match_rust_catalog_and_state_still_denies_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:568` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:597` | no direct lock; permission/admission/call site (test fixture/barrier; db_grants_deny_unknown_targets_and_database_rejects_unknown_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:614` | no direct lock; permission/admission/call site (test fixture/barrier; db_grants_deny_unknown_targets_and_database_rejects_unknown_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:635` | no direct lock; permission/admission/call site (test fixture/barrier; db_grants_deny_unknown_targets_and_database_rejects_unknown_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:813` | no direct lock; permission/admission/call site (test fixture/barrier; game_roles_have_exact_contract_permissions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:826` | no direct lock; permission/admission/call site (test fixture/barrier; game_flags_deny_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:827` | no direct lock; permission/admission/call site (test fixture/barrier; game_flags_deny_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:848` | no direct lock; permission/admission/call site (test fixture/barrier; game_flags_deny_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:856` | no direct lock; permission/admission/call site (test fixture/barrier; game_flags_deny_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:865` | no direct lock; permission/admission/call site (test fixture/barrier; db_game_roles_authorize_only_explicit_grants_and_deploy_stays_dormant) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:887` | no direct lock; permission/admission/call site (test fixture/barrier; db_game_roles_authorize_only_explicit_grants_and_deploy_stays_dormant) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:901` | no direct lock; permission/admission/call site (test fixture/barrier; db_game_roles_authorize_only_explicit_grants_and_deploy_stays_dormant) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:909` | no direct lock; permission/admission/call site (test fixture/barrier; db_game_roles_authorize_only_explicit_grants_and_deploy_stays_dormant) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:948` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_two_queued_mutations_do_not_starve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:950` | pg_advisory_xact_lock: 71010 (test fixture/barrier; db_transaction_authorization_two_queued_mutations_do_not_starve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:960` | pg_advisory_xact_lock: 71010 (test fixture/barrier; db_transaction_authorization_two_queued_mutations_do_not_starve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:961` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_two_queued_mutations_do_not_starve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:962` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_two_queued_mutations_do_not_starve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1005` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_one_connection_identity_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1012` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_one_connection_identity_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1025` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_one_connection_identity_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1047` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_observes_revocation_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1061` | pg_advisory_xact_lock: 71010 (test fixture/barrier; db_transaction_authorization_observes_revocation_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1066` | no direct lock; permission/admission/call site (test fixture/barrier; db_transaction_authorization_observes_revocation_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:8` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:24` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:25` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:26` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:254` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:271` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:283` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:288` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:295` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:296` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:321` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:326` | no direct lock; permission/admission/call site (production; execute) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:6` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:12` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:68` | no direct lock; permission/admission/call site (test fixture/barrier; pagination_admin_queue_caps_and_empty) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:112` | no direct lock; permission/admission/call site (test fixture/barrier; missing_backend_denies_reads) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:118` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:190` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_support_atomic_audit_and_refusal) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:191` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_support_atomic_audit_and_refusal) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:16` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:54` | no direct lock; permission/admission/call site (production; configured_router_from) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:81` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/test_staff.rs:72` | FOR UPDATE: accounts (test fixture/barrier; race_account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/test_staff.rs:88` | pg_try_advisory_xact_lock: 71010 (test fixture/barrier; race_account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:5` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:84` | no direct lock; permission/admission/call site (production; resolve) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:92` | no direct lock; permission/admission/call site (production; authenticated_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:95` | no direct lock; permission/admission/call site (production; authenticated_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:101` | FOR SHARE: accounts (production; state_in) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:137` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:196` | no direct lock; permission/admission/call site (production; configured_guard_from) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:214` | no direct lock; permission/admission/call site (production; guarded) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:241` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:380` | no direct lock; permission/admission/call site (production; authenticate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:382` | no direct lock; permission/admission/call site (production; authenticate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:384` | no direct lock; permission/admission/call site (production; authenticate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:470` | no direct lock; permission/admission/call site (production; permission_catalog) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:496` | no direct lock; permission/admission/call site (production; holder_batch) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:506` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:647` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:652` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:655` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:656` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:664` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:673` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:679` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:688` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web.rs:708` | ordered accounts UPDATE → RBAC exclusive → session/MFA UPDATE → role effects; final actor revalidation (production; revoke) | 3 → 5 → 6 → 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/admin/src/web_tests.rs:18` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:24` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:56` | no direct lock; permission/admission/call site (test fixture/barrier; feature) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:186` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:198` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:203` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:249` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:359` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:365` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:376` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:417` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:428` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:439` | no direct lock; permission/admission/call site (test fixture/barrier; database_navigation_authorization_csrf_and_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/bestiary/mod.rs:94` | no direct lock; permission/admission/call site (production; spawn_policy) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/bestiary/web.rs:106` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns.rs:185` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns.rs:245` | FOR UPDATE: accounts (production; add_marker) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns.rs:295` | FOR UPDATE: accounts (production; report) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns_tests.rs:107` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_spawns_private_markers_and_reports) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns_tests.rs:274` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_spawns_private_markers_and_reports) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns_tests.rs:293` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_spawns_private_markers_and_reports) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/README.md:13` | FOR UPDATE (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/bounded.rs:19` | FOR UPDATE: bazaar_offers (production; offers) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:76` | no direct lock; permission/admission/call site (production; fmt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:211` | no direct lock; permission/admission/call site (production; account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:255` | no direct lock; permission/admission/call site (production; detail) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:271` | no direct lock; permission/admission/call site (production; sell) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:304` | no direct lock; permission/admission/call site (production; create) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:344` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:465` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/service.rs:108` | pg_advisory_xact_lock: 59 (production; request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/characters/service.rs:176` | FOR UPDATE: bazaar_listings (production; publish) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/characters/service.rs:233` | FOR SHARE: bazaar_listings (production; detail) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/characters/service.rs:314` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/characters/tests.rs:111` | FOR UPDATE: test_accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/tests.rs:133` | FOR UPDATE: test_funds (test fixture/barrier; lock_accounts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/tests.rs:432` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_unknown_or_revoked_game_evidence_hides_and_denies) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority.rs:93` | FOR UPDATE: bazaar_game_issuer (production; lock) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority.rs:120` | FOR UPDATE: bazaar_game_outbox (production; receive) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_evidence.rs:32` | FOR UPDATE: bazaar_resync_progress (production; finish_resync) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_exchange.rs:205` | no direct lock; permission/admission/call site (production; compensation_ready) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_http.rs:126` | no direct lock; permission/admission/call site (production; result) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_http.rs:250` | no direct lock; permission/admission/call site (production; select_receipt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:225` | FOR UPDATE: bazaar_game_issuer (test fixture/barrier; postgres_exact_receipt_replay_conflict_audit_rollback_and_nonrollback_floor) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:671` | FOR UPDATE: bazaar_game_issuer (test fixture/barrier; item_wallet_scenario) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:1580` | FOR UPDATE: bazaar_listings (test fixture/barrier; postgres_receipt_evidence_and_worker_listing_lock_do_not_deadlock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:1686` | pg_advisory_xact_lock: 22005160000 (test fixture/barrier; postgres_legacy_staged_transfer_publishes_exact_bytes_after_newer_revision) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:1933` | pg_advisory_xact_lock: 22005160000 (test fixture/barrier; postgres_private_transfer_barrier_fences_all_character_issuers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2110` | no direct lock; permission/admission/call site (test fixture/barrier; bzb_receipt_admission_precedes_oversized_version_selection) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2599` | no direct lock; permission/admission/call site (test fixture/barrier; gold_release_retry_scenario) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2938` | FOR UPDATE: bazaar_game_issuer (test fixture/barrier; l6_04_suspended_leader_can_be_outbid_and_exact_hold_released) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2985` | no direct lock; permission/admission/call site (test fixture/barrier; l6_04_suspended_leader_can_be_outbid_and_exact_hold_released) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2994` | no direct lock; permission/admission/call site (test fixture/barrier; l6_04_suspended_leader_can_be_outbid_and_exact_hold_released) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:3009` | no direct lock; permission/admission/call site (test fixture/barrier; l6_04_suspended_leader_can_be_outbid_and_exact_hold_released) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:30` | FOR SHARE: accounts (production; safe_return) | 3 | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:46` | FOR SHARE: accounts (production; state) | 3 | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:56` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:92` | FOR SHARE: accounts (production; account) | 3 | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:158` | FOR UPDATE: wallet_accounts (production; lock_accounts) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:177` | FOR UPDATE: bazaar_coin_holds (production; reserve) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:195` | no direct lock; permission/admission/call site (production; reserve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:198` | no direct lock; permission/admission/call site (production; reserve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:224` | FOR UPDATE: bazaar_coin_holds (production; release) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:241` | FOR UPDATE: bazaar_coin_dispositions (production; release) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:322` | FOR UPDATE: bazaar_coin_dispositions (production; settle) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:339` | no direct lock; permission/admission/call site (production; settle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:344` | FOR UPDATE: bazaar_coin_holds (production; settle) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:374` | FOR UPDATE: bazaar_coin_dispositions (production; complete) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:383` | FOR UPDATE: bazaar_coin_holds (production; complete) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:409` | no direct lock; permission/admission/call site (production; complete) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports_lock_tests.rs:77` | FOR UPDATE: accounts (test fixture/barrier; postgres_bazaar_locks_complete_identity_set_before_wallets) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports_lock_tests.rs:107` | FOR UPDATE NOWAIT: wallet_accounts (test fixture/barrier; postgres_bazaar_locks_complete_identity_set_before_wallets) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_projection.rs:249` | FOR UPDATE: bazaar_game_issuer (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_projection.rs:284` | FOR UPDATE: bazaar_game_projections (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_store.rs:17` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_store.rs:93` | FOR UPDATE: bazaar_game_issuer (production; stage) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_store.rs:152` | FOR UPDATE: bazaar_game_issuer (production; reopen) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/mod.rs:6` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/characters.rs:507` | pg_advisory_xact_lock: 22005160000 (production; reconcile_transfer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs:90` | FOR UPDATE: bazaar_listings (production; accept_gold_bid) | 2 | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs:95` | FOR UPDATE: bazaar_authority_gold_bids (production; accept_gold_bid) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:46` | FOR UPDATE: bazaar_authority_snapshot_keys (production; configured) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:346` | FOR UPDATE: bazaar_listings (production; bid) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:366` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:380` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:486` | FOR UPDATE: bazaar_listings (production; control) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:550` | pg_advisory_xact_lock: 64004 (production; check_bid_request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/tls.rs:313` | FOR UPDATE: bazaar_game_issuer (test fixture/barrier; postgres_exchange_deadline_cancels_issuer_wait_and_releases_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/tls.rs:318` | FOR UPDATE: bazaar_game_issuer (test fixture/barrier; postgres_exchange_deadline_cancels_issuer_wait_and_releases_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/transport.rs:136` | no direct lock; permission/admission/call site (production; event) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/worker.rs:141` | FOR UPDATE: bazaar_listings (production; advance) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/worker.rs:393` | pg_advisory_xact_lock: 22005160000 (production; transfer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/game/runtime/worker.rs:682` | pg_advisory_xact_lock: 22005160000 (production; publish_transfer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/README.md:37` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:29` | no direct lock; permission/admission/call site (production; enabled) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:69` | no direct lock; permission/admission/call site (production; parties) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:82` | no direct lock; permission/admission/call site (production; create) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:84` | pg_advisory_xact_lock: 60 (production; create) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:128` | no direct lock; permission/admission/call site (production; publish) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:164` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:248` | no direct lock; permission/admission/call site (production; offer) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:274` | pg_advisory_xact_lock: 57 (production; offer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:328` | FOR UPDATE: bazaar_offers (production; accept) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:339` | no direct lock; permission/admission/call site (production; accept) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:437` | FOR UPDATE: bazaar_offers (production; reject) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:446` | no direct lock; permission/admission/call site (production; reject) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:481` | no direct lock; permission/admission/call site (production; buy_now) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:566` | no direct lock; permission/admission/call site (production; cancel) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:603` | no direct lock; permission/admission/call site (production; renew) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:692` | no direct lock; permission/admission/call site (production; owner_view) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/service.rs:739` | FOR UPDATE: bazaar_listings, bazaar_item_details (production; locked) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/items/tests.rs:88` | pg_advisory_xact_lock: 600 (test fixture/barrier; lock_accounts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/locking.rs:7` | pg_advisory_xact_lock_shared: 64002 (production; listing_gate) | 1 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/locking.rs:19` | FOR SHARE: accounts (production; accounts) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:119` | no direct lock; permission/admission/call site (production; enabled) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:182` | no direct lock; permission/admission/call site (production; report) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:184` | pg_advisory_xact_lock: 64 (production; report) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:242` | no direct lock; permission/admission/call site (production; promote) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:326` | FOR UPDATE: bazaar_listings (production; lock_listing) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:341` | no direct lock; permission/admission/call site (production; held) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:405` | FOR UPDATE (production; lock_staff_target) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:408` | pg_advisory_xact_lock: 64002 (production; lock_staff_target) | 1 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:425` | FOR UPDATE: bazaar_invalidation_requests (production; lock_staff_target) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:431` | FOR UPDATE: bazaar_reports (production; lock_staff_target) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:438` | pg_advisory_xact_lock: 64001 (production; lock_staff_target) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:444` | pg_advisory_xact_lock: 64002 (production; lock_staff_target) | 1 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:502` | FOR UPDATE: bazaar_invalidation_requests (production; execute_staff) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:530` | FOR UPDATE: bazaar_reports (production; execute_staff) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:557` | pg_advisory_xact_lock: 64001 (production; execute_staff) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation.rs:583` | pg_advisory_xact_lock: 64002 (production; execute_staff) | 1 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/moderation_en.ftl:38` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:654` | FOR UPDATE: bazaar_listings (test fixture/barrier; postgres_restriction_does_not_wait_for_or_lock_listing_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/notify.rs:207` | FOR UPDATE SKIP LOCKED: bazaar_notification_outbox (production; dispatch_one) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/notify/tests.rs:306` | pg_advisory_xact_lock: 64006 (test fixture/barrier; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/notify/tests.rs:328` | pg_advisory_xact_lock: 64006 (test fixture/barrier; postgres_notification_record_and_dispatch_share_evidence_lock_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:45` | no direct lock; permission/admission/call site (production; currency) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:70` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:80` | no direct lock; permission/admission/call site (production; authorize_many) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:110` | no direct lock; permission/admission/call site (production; create) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:141` | no direct lock; permission/admission/call site (production; publish) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:164` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:170` | no direct lock; permission/admission/call site (production; bid) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:188` | no direct lock; permission/admission/call site (production; bid_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:282` | no direct lock; permission/admission/call site (production; offer) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:306` | pg_advisory_xact_lock: 57 (production; offer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:378` | FOR UPDATE: bazaar_offers (production; accept_offer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:401` | no direct lock; permission/admission/call site (production; accept_offer) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:458` | FOR UPDATE: bazaar_offers (production; reject_offer) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:462` | no direct lock; permission/admission/call site (production; reject_offer) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:487` | no direct lock; permission/admission/call site (production; cancel) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:562` | FOR UPDATE: bazaar_settlements (production; settle) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:600` | no direct lock; permission/admission/call site (production; settle) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:666` | FOR UPDATE: bazaar_listings (production; locked) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/repository.rs:681` | FOR UPDATE: bazaar_settlements (production; settlement) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:177` | no direct lock; permission/admission/call site (production; choice) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:207` | no direct lock; permission/admission/call site (production; visible) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:297` | no direct lock; permission/admission/call site (production; enabled) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:329` | no direct lock; permission/admission/call site (production; search) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:411` | no direct lock; permission/admission/call site (production; watch) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:430` | pg_advisory_xact_lock: 64005 (production; watch) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:468` | no direct lock; permission/admission/call site (production; save) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:470` | pg_advisory_xact_lock: 61 (production; save) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:495` | no direct lock; permission/admission/call site (production; saved) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search.rs:518` | no direct lock; permission/admission/call site (production; delete_saved) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/bazaar/src/search_tests.rs:25` | no direct lock; permission/admission/call site (test fixture/barrier; candidate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search_tests.rs:95` | no direct lock; permission/admission/call site (test fixture/barrier; freshness_and_revocation_fail_closed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search_tests.rs:97` | no direct lock; permission/admission/call site (test fixture/barrier; freshness_and_revocation_fail_closed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/fixtures/plat3/README.md:76` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:50` | no direct lock; permission/admission/call site (test fixture/barrier; anti_sniping_adds_five_minutes_once_per_request) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:98` | no direct lock; permission/admission/call site (test fixture/barrier; settlement_commission_is_exact_and_proceeds_wait_for_delivery) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:141` | no direct lock; permission/admission/call site (test fixture/barrier; adapter_failure_rolls_back_ledger_settlement_listing_and_audit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:350` | no direct lock; permission/admission/call site (test fixture/barrier; expiry_releases_offers_and_cancellation_requires_no_activity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:365` | no direct lock; permission/admission/call site (test fixture/barrier; expiry_releases_offers_and_cancellation_requires_no_activity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:469` | no direct lock; permission/admission/call site (test fixture/barrier; terminal_cancellation_and_expiry_require_return_evidence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:520` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_core_legacy_offer_cleanup_keeps_every_reservation_accounted_for) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:546` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_committed_delivery_completes_after_trade_restriction) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:581` | FOR SHARE: accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:625` | pg_advisory_xact_lock: 64002 (test fixture/barrier; creation_waits_for_moderation_before_locking_its_account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:647` | FOR UPDATE NOWAIT: accounts (test fixture/barrier; creation_waits_for_moderation_before_locking_its_account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:666` | FOR SHARE: accounts (test fixture/barrier; lock_accounts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:728` | FOR UPDATE: accounts (test fixture/barrier; bid_prepares_previous_leader_before_actor_account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:753` | FOR UPDATE NOWAIT: accounts (test fixture/barrier; bid_prepares_previous_leader_before_actor_account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:767` | pg_advisory_xact_lock: 64002 (test fixture/barrier; postgres_drc_listing_gate_has_shared_budget) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:808` | pg_advisory_xact_lock: 64002 (test fixture/barrier; postgres_drc_trigger_gate_has_shared_budget) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/support/mod.rs:151` | FOR UPDATE: test_ledger (test fixture/barrier; load) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/support/mod.rs:326` | no direct lock; permission/admission/call site (test fixture/barrier; listing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/http.rs:67` | no direct lock; permission/admission/call site (production; private) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/http.rs:97` | no direct lock; permission/admission/call site (production; show_inner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/http.rs:238` | no direct lock; permission/admission/call site (production; save_inner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/mod.rs:207` | FOR UPDATE: accounts (production; begin) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:143` | FOR SHARE: accounts (production; issue) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:151` | FOR SHARE: accounts (production; issue) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:166` | pg_advisory_xact_lock_shared: 71010 (production; issue) | 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:176` | FOR UPDATE: character_bootstrap_intent_authority (production; issue) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:182` | FOR SHARE: account_roles, role_permissions, accounts (production; issue) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:63` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:64` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:86` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:93` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:416` | no direct lock; permission/admission/call site (test fixture/barrier; operator_issuance_audits_once_without_the_intent_body) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:431` | no direct lock; permission/admission/call site (test fixture/barrier; unauthorized_actor_issues_and_audits_nothing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:481` | FOR UPDATE: character_bootstrap_intent_authority (test fixture/barrier; issuer_authority_wait_uses_shared_mutation_deadline) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/tests.rs:114` | no direct lock; permission/admission/call site (test fixture/barrier; identity_bad_request_and_default_off) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/README.md:4` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/README.md:26` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/http.rs:109` | no direct lock; permission/admission/call site (production; answer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:3` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:151` | no direct lock; permission/admission/call site (production; available) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:152` | no direct lock; permission/admission/call site (production; available) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:186` | no direct lock; permission/admission/call site (production; with_restore_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:200` | no direct lock; permission/admission/call site (production; enqueue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:210` | no direct lock; permission/admission/call site (production; enqueue_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/services.rs:68` | no direct lock; permission/admission/call site (production; binding) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/services.rs:70` | no direct lock; permission/admission/call site (production; binding) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/services.rs:83` | pg_advisory_xact_lock: 22005160000 (production; enqueue_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:14` | no direct lock; permission/admission/call site (production; status) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:52` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:56` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:67` | pg_advisory_xact_lock: 22005160000 (production; issue_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:133` | no direct lock; permission/admission/call site (production; source) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:229` | FOR SHARE: character_restore_locks (production; publish) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/storage.rs:236` | FOR UPDATE: character_commands (production; publish) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/commands/tests.rs:238` | no direct lock; permission/admission/call site (test fixture/barrier; guard_denies_wrong_identity_disabled_surface_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/tests.rs:267` | no direct lock; permission/admission/call site (test fixture/barrier; guard_denies_wrong_identity_disabled_surface_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:75` | no direct lock; permission/admission/call site (production; from) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:102` | no direct lock; permission/admission/call site (production; eligible) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:104` | no direct lock; permission/admission/call site (production; eligible) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:108` | no direct lock; permission/admission/call site (production; eligible_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:120` | no direct lock; permission/admission/call site (production; limit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:122` | no direct lock; permission/admission/call site (production; limit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:141` | no direct lock; permission/admission/call site (production; worlds) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:143` | no direct lock; permission/admission/call site (production; worlds) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:177` | no direct lock; permission/admission/call site (production; reserved) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:179` | no direct lock; permission/admission/call site (production; reserved) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:200` | FOR NO KEY UPDATE: accounts (production; submit) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:207` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:216` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:221` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:228` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:235` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/service.rs:240` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/create_web/tests.rs:574` | FOR UPDATE: accounts (test fixture/barrier; db_all_pool_connections_can_submit_concurrently) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/http.rs:161` | no direct lock; permission/admission/call site (production; bounds_and_rate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/http.rs:234` | no direct lock; permission/admission/call site (production; refusal) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/mod.rs:18` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:76` | no direct lock; permission/admission/call site (production; publication) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:91` | FOR UPDATE: account_character_snapshots (production; snapshot) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:165` | FOR SHARE: account_character_feed (production; list_characters) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:220` | FOR UPDATE: account_character_feed (production; advance_epoch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:308` | no direct lock; permission/admission/call site (test fixture/barrier; peer_flag_and_transport_failures_need_no_database) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:332` | no direct lock; permission/admission/call site (test fixture/barrier; peer_flag_and_transport_failures_need_no_database) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:397` | no direct lock; permission/admission/call site (test fixture/barrier; db_golden_http_acknowledgements_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:614` | FOR UPDATE: account_character_feed (test fixture/barrier; db_concurrent_snapshots_wait_for_feed_lock_and_finish_at_highest_revision) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/overview/wallet.rs:11` | FOR SHARE: accounts (production; state) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/overview/wallet.rs:24` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/README.md:35` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/README.md:45` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:2` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:97` | FOR SHARE: account_character_feed (production; lock) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:222` | FOR SHARE: accounts (production; edit) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:271` | FOR UPDATE: accounts (production; save) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:339` | FOR SHARE: accounts (production; read_public_reference) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:365` | FOR SHARE: accounts (production; read_main) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/http.rs:117` | no direct lock; permission/admission/call site (production; failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/http.rs:188` | no direct lock; permission/admission/call site (production; exchange_ready) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:45` | no direct lock; permission/admission/call site (production; available) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:46` | no direct lock; permission/admission/call site (production; available) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:82` | no direct lock; permission/admission/call site (production; notice) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/storage.rs:7` | FOR UPDATE: character_restore_locks (production; lock) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/storage.rs:9` | FOR SHARE: character_restore_locks (production; lock) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/tests.rs:184` | no direct lock; permission/admission/call site (test fixture/barrier; guards_refuse_wrong_identity_bad_schema_source_and_disabled) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/tests.rs:212` | no direct lock; permission/admission/call site (test fixture/barrier; guards_refuse_wrong_identity_bad_schema_source_and_disabled) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/browser.rs:70` | no direct lock; permission/admission/call site (production; error) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:79` | no direct lock; permission/admission/call site (production; safe_return) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:87` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:96` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:147` | pg_advisory_xact_lock: 22005160000 (production; request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:153` | pg_advisory_xact_lock: 47 (production; request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:159` | pg_advisory_xact_lock: 46 (production; request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:191` | FOR UPDATE: character_service_orders (production; request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:207` | no direct lock; permission/admission/call site (production; request) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:215` | pg_advisory_xact_lock: 22005160000 (production; request) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:228` | no direct lock; permission/admission/call site (production; request) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:262` | no direct lock; permission/admission/call site (production; receive) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:264` | FOR UPDATE: character_commands, character_service_orders (production; receive) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:293` | no direct lock; permission/admission/call site (production; receive) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:323` | FOR UPDATE: character_service_orders (production; receive) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:356` | FOR SHARE: character_restore_locks (production; restore_lock) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:380` | no direct lock; permission/admission/call site (production; source) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/projection.rs:139` | FOR UPDATE: account_character_feed (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:151` | FOR UPDATE: character_service_feed (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:170` | FOR UPDATE: character_service_states (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:197` | FOR SHARE: account_character_feed, character_service_feed, character_service_states (production; owner_state) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:244` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_reservation_insufficient_funds_and_transaction_rollback) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:268` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_concurrent_reservations_cannot_overspend) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:302` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_settlement_replay_conflict_and_concurrency) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:313` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_settlement_replay_conflict_and_concurrency) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:327` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_settlement_replay_conflict_and_concurrency) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:335` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_settlement_replay_conflict_and_concurrency) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:638` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_v1_v2_isolation_identity_and_receipt_binding) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:662` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_v1_v2_isolation_identity_and_receipt_binding) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/transport.rs:136` | no direct lock; permission/admission/call site (production; status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/transport.rs:250` | no direct lock; permission/admission/call site (production; router_with_restore) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths.rs:166` | no direct lock; permission/admission/call site (production; snapshot) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths.rs:200` | no direct lock; permission/admission/call site (production; watermark) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/README.md:34` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/storage.rs:25` | FOR UPDATE: public_projection_state (production; fence) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/tests.rs:178` | no direct lock; permission/admission/call site (test fixture/barrier; transport_refusal_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/web.rs:185` | FOR SHARE: public_projection_state (production; load_inner) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families.rs:222` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families.rs:272` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/storage.rs:22` | FOR UPDATE: public_projection_state (production; fence) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/storage.rs:51` | FOR UPDATE: public_character_families_v2 (production; snapshot) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/storage.rs:113` | FOR UPDATE: public_character_families_v2_watermarks (production; watermark) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:288` | no direct lock; permission/admission/call site (test fixture/barrier; ingest_refuses_when_disabled_or_without_the_peer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:542` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_owner_only_families_serve_only_the_current_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:543` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_owner_only_families_serve_only_the_current_owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:952` | LOCK TABLE: public_character_families_v2 (test fixture/barrier; audit_w2_c02_owner_body_wait_rechecks_epoch_for_every_family) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/web.rs:317` | FOR SHARE: public_projection_state (production; public_skills) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/web.rs:431` | FOR SHARE: public_projection_state (production; owner_rows) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/guilds.rs:106` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/guilds.rs:143` | FOR UPDATE: public_projection_state (production; apply) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/guilds_pages.rs:305` | FOR SHARE: public_projection_state (production; load) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/guilds_tests.rs:57` | no direct lock; permission/admission/call site (test fixture/barrier; refusal_and_bad_wire) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses.rs:315` | FOR SHARE: public_projection_state (production; load) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses_ingest.rs:32` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses_ingest.rs:71` | FOR UPDATE: public_projection_state (production; apply) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses_tests.rs:136` | no direct lock; permission/admission/call site (test fixture/barrier; routes_refuse_disabled_wrong_peer_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest.rs:175` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest.rs:215` | FOR UPDATE: public_projection_state (production; apply) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest/ingest_tests.rs:55` | no direct lock; permission/admission/call site (test fixture/barrier; disabled_identity_limits_and_malformed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest_budget.rs:44` | FOR UPDATE: community_ingest_budgets (production; admit) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/online.rs:174` | FOR SHARE: public_projection_state (production; load_inner) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:76` | FOR SHARE: public_projection_state (production; character) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:111` | no direct lock; permission/admission/call site (production; moderator) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:115` | no direct lock; permission/admission/call site (production; moderator) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:116` | no direct lock; permission/admission/call site (production; moderator) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:147` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:167` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:64` | pg_advisory_xact_lock: 83 (production; create) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:93` | pg_advisory_xact_lock: 83 (production; report) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:97` | FOR UPDATE: pd_party_posts (production; report) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:168` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; moderate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:173` | FOR UPDATE: pd_party_posts (production; moderate) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:183` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; moderate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:184` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; moderate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:220` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; moderate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:221` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; moderate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_store.rs:223` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; moderate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/community/src/party/party_tests.rs:108` | no direct lock; permission/admission/call site (test fixture/barrier; moderate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:111` | no direct lock; permission/admission/call site (test fixture/barrier; moderate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:459` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:465` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:532` | no direct lock; permission/admission/call site (test fixture/barrier; db_staff_guard_and_audit_failure_roll_back) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:691` | no direct lock; permission/admission/call site (test fixture/barrier; audit_w2_c05_production_authority_owner_report_and_staff_flow) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:750` | no direct lock; permission/admission/call site (test fixture/barrier; delta_party_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:777` | FOR UPDATE: pd_party_posts (test fixture/barrier; delta_party_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:819` | no direct lock; permission/admission/call site (test fixture/barrier; delta_party_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:823` | no direct lock; permission/admission/call site (test fixture/barrier; delta_party_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:831` | no direct lock; permission/admission/call site (test fixture/barrier; delta_party_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/privacy.md:18` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/privacy.md:65` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/profiles.rs:240` | no direct lock; permission/admission/call site (production; load) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:6` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals.rs:174` | FOR KEY SHARE: accounts (production; invite) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals.rs:184` | pg_advisory_xact_lock: 5600 (production; invite) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals.rs:256` | FOR UPDATE: referral_invitations (production; claim) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals.rs:293` | no direct lock; permission/admission/call site (production; claim) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals.rs:343` | no direct lock; permission/admission/call site (production; notify) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:44` | FOR UPDATE: referral_activity (production; allowed) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:67` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:408` | FOR UPDATE SKIP LOCKED: referral_campaign_rewards (production; process) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:413` | no direct lock; permission/admission/call site (production; process) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:427` | pg_advisory_xact_lock: 5601 (production; process) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:430` | FOR UPDATE: referral_email_attempts, referral_email_invitations, accounts (production; process) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/tests.rs:43` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1440` | FOR KEY SHARE NOWAIT: accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1453` | pg_try_advisory_xact_lock: 5600 (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1549` | FOR UPDATE: accounts (test fixture/barrier; nwave_first_link_locks_account_before_qualification) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1665` | FOR UPDATE: premium_authority (test fixture/barrier; nwave_stream_locks_existing_authority_before_qualification) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1697` | FOR UPDATE: accounts (test fixture/barrier; nwave_accept_locks_inviter_before_qualification) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1733` | FOR UPDATE: accounts (test fixture/barrier; nwave_snapshot_locks_inviter_before_qualification) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1790` | FOR UPDATE: accounts (test fixture/barrier; nwave_signup_and_email_lock_accounts_before_qualification) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2211` | FOR KEY SHARE NOWAIT: accounts (test fixture/barrier; nwave_preparation_prevents_account_and_authority_lock_upgrades_after_5600) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2212` | FOR KEY SHARE NOWAIT: premium_authority (test fixture/barrier; nwave_preparation_prevents_account_and_authority_lock_upgrades_after_5600) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2249` | FOR UPDATE: accounts (test fixture/barrier; nwave_verification_contention_preserves_attribution_for_retry) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2273` | FOR UPDATE NOWAIT: accounts (test fixture/barrier; nwave_verification_contention_preserves_attribution_for_retry) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2309` | FOR UPDATE: accounts (test fixture/barrier; nwave_session_contention_aborts_login_before_first_purchase) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2331` | FOR UPDATE NOWAIT: accounts (test fixture/barrier; nwave_session_contention_aborts_login_before_first_purchase) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2457` | no direct lock; permission/admission/call site (test fixture/barrier; l6_01_pending_termination_refuses_campaign_rewards) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals_tests.rs:43` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today.rs:60` | no direct lock; permission/admission/call site (production; from_lookup) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/README.md:52` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/http.rs:100` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/http.rs:132` | no direct lock; permission/admission/call site (production; status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/store.rs:18` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/store.rs:23` | FOR UPDATE: public_projection_state (production; apply) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/store.rs:37` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/view.rs:171` | FOR SHARE: public_projection_state (production; read_inner) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/view.rs:174` | FOR SHARE: today_snapshots (production; read_inner) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/v2_watermark.rs:107` | no direct lock; permission/admission/call site (production; admission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/v2_watermark.rs:270` | no direct lock; permission/admission/call site (test fixture/barrier; disabled_and_wrong_peer_refuse_before_body_poll) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:198` | no direct lock; permission/admission/call site (test fixture/barrier; today_transport_refusals_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:209` | no direct lock; permission/admission/call site (test fixture/barrier; today_transport_refusals_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:457` | no direct lock; permission/admission/call site (test fixture/barrier; today_postgres_bad_schema_scope_and_unknown_content) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:746` | LOCK TABLE: world_admin_names (test fixture/barrier; audit06_wait_past_interval_refuses_boost_data) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:795` | LOCK TABLE: world_admin_names (test fixture/barrier; audit_w2_c02_today_catalogue_wait_revalidates_invalidations) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/worlds.rs:323` | LOCK TABLE: public_snapshots (test fixture/barrier; audit06_world_database_wait_rechecks_serving_clock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/worlds.rs:342` | no direct lock; permission/admission/call site (test fixture/barrier; audit06_world_database_wait_rechecks_serving_clock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/worlds.rs:361` | no direct lock; permission/admission/call site (test fixture/barrier; audit_w2_c04_c07_presence_never_authorizes_runtime_or_cached_claims) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:116` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:122` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:128` | FOR UPDATE: announcements (production; save) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:138` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:141` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:142` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:149` | FOR UPDATE: announcements (production; save) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:193` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements.rs:194` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/announcements_staff.rs:18` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:36` | no direct lock; permission/admission/call site (production; staff_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:234` | no direct lock; permission/admission/call site (production; persist) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:13` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:19` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:109` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_announcements_and_today) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:117` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_announcements_and_today) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:404` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_announcement_dispatch) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:405` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_announcement_dispatch) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:417` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_announcement_dispatch) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:597` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_announcement_dispatch) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:83` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:89` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:95` | FOR UPDATE: events (production; save) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:105` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:108` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:109` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:115` | FOR UPDATE: events (production; save) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:157` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:158` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:165` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:172` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:177` | FOR UPDATE: events (production; transition) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:187` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:188` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:189` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:193` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:194` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:195` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:198` | FOR UPDATE: events (production; transition) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:238` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:239` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:240` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/repository.rs:242` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/events/tests.rs:9` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:32` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:38` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:156` | no direct lock; permission/admission/call site (test fixture/barrier; close) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:158` | no direct lock; permission/admission/call site (test fixture/barrier; author) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:166` | no direct lock; permission/admission/call site (test fixture/barrier; author) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:228` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_publication_versions_validation_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:246` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_publication_versions_validation_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:251` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_publication_versions_validation_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:510` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_events_refuse_session_revoked_after_authentication) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:13` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:24` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:164` | no direct lock; permission/admission/call site (production; show) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:180` | no direct lock; permission/admission/call site (production; admin_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:185` | no direct lock; permission/admission/call site (production; staff_routes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:373` | no direct lock; permission/admission/call site (production; form) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:142` | no direct lock; permission/admission/call site (production; configured_router_from) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:342` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; upload) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:348` | pg_advisory_xact_lock: 12 (production; upload) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:357` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; upload) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:367` | pg_advisory_xact_lock: 12 (production; upload) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:372` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; upload) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:391` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; upload) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:392` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; upload) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:473` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:478` | FOR UPDATE: media (production; delete) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:487` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:495` | FOR UPDATE: media (production; delete) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:502` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:537` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media.rs:538` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/media_tests.rs:88` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_media_atomic_storage_references_and_integrity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:97` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_media_atomic_storage_references_and_integrity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:208` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:214` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:237` | no direct lock; permission/admission/call site (test fixture/barrier; test_app) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:249` | no direct lock; permission/admission/call site (test fixture/barrier; browser_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:425` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_processing_releases_rbac_for_other_mutations) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:442` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_processing_releases_rbac_for_other_mutations) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:445` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_processing_releases_rbac_for_other_mutations) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:453` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_processing_releases_rbac_for_other_mutations) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:152` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:435` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:440` | FOR UPDATE: news (production; save) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:449` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:457` | FOR UPDATE: news (production; save) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:466` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:503` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:504` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; save) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:532` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:537` | FOR UPDATE: news (production; delete) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:546` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:554` | FOR UPDATE: news (production; delete) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:560` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:590` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news.rs:591` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; delete) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/news_tests.rs:56` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:65` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:211` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:268` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:277` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:283` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:298` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:312` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_news_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:449` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:455` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:17` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:34` | no direct lock; permission/admission/call site (production; staff_router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:248` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; persist) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:254` | pg_advisory_xact_lock: 71102 (production; persist) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:263` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; persist) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:267` | pg_advisory_xact_lock: 71102 (production; persist) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:271` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; persist) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:349` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; persist) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/staff.rs:350` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; persist) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/content/src/pages/tests.rs:52` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pages_publication_and_legal_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:57` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pages_publication_and_legal_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:259` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pages_publication_and_legal_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:282` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pages_publication_and_legal_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:454` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:460` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:67` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_staff_libraries_reach_records_beyond_each_cap) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:115` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_staff_libraries_reach_records_beyond_each_cap) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:2` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:11` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:17` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:69` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:95` | no direct lock; permission/admission/call site (test fixture/barrier; event_save_and_transition_complete_with_one_connection) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:134` | no direct lock; permission/admission/call site (test fixture/barrier; announcement_save_completes_with_one_connection) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:232` | no direct lock; permission/admission/call site (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:240` | no direct lock; permission/admission/call site (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:285` | FOR UPDATE: news (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:300` | pg_advisory_xact_lock: 71102, fenced-page (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:310` | pg_advisory_xact_lock: 12 (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:327` | FOR UPDATE: media (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:349` | no direct lock; permission/admission/call site (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:363` | no direct lock; permission/admission/call site (test fixture/barrier; content_mutations_recheck_after_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:408` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_mutations_require_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:413` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_mutations_require_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:434` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_mutations_require_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:448` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_mutations_bound_fence_and_resource_waits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:458` | FOR UPDATE: announcements (test fixture/barrier; editorial_mutations_bound_fence_and_resource_waits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:463` | FOR UPDATE: events (test fixture/barrier; editorial_mutations_bound_fence_and_resource_waits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:469` | pg_advisory_xact_lock: 71010 (test fixture/barrier; editorial_mutations_bound_fence_and_resource_waits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:519` | no direct lock; permission/admission/call site (test fixture/barrier; announcement_unchanged_and_metadata_saves_preserve_translation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:585` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_writers_reject_grant_expired_during_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:608` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_writers_reject_grant_expired_during_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:618` | FOR UPDATE: announcements (test fixture/barrier; editorial_writers_reject_grant_expired_during_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:620` | FOR UPDATE: events (test fixture/barrier; editorial_writers_reject_grant_expired_during_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:635` | no direct lock; permission/admission/call site (test fixture/barrier; editorial_writers_reject_grant_expired_during_resource_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/db/src/outbox.rs:75` | FOR UPDATE SKIP LOCKED (production; claim_with_deadline) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/db/src/outbox.rs:89` | FOR UPDATE SKIP LOCKED (production; claim_with_deadline) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/db/tests/store_offers_coins_only.rs:139` | pg_advisory_unlock_all (test fixture/barrier; transferable_offer_stops_the_migration_and_changes_nothing) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/command.rs:106` | no direct lock; permission/admission/call site (production; read_bounded) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/downloads/src/command.rs:118` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; authorize_in) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/downloads/src/command.rs:132` | no direct lock; permission/admission/call site (production; may_publish) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/downloads/src/domain.rs:55` | no direct lock; permission/admission/call site (production; may_publish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/domain.rs:56` | no direct lock; permission/admission/call site (production; may_publish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/storage.rs:44` | no direct lock; permission/admission/call site (production; enabled) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/downloads/src/storage.rs:68` | accounts UPDATE → channel UPDATE → RBAC exclusive (production; publish) | 3 → 4 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/downloads/src/storage.rs:81` | FOR UPDATE: client_channels (production; publish) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/downloads/src/storage.rs:92` | accounts UPDATE → channel UPDATE → RBAC exclusive (production; publish) | 3 → 4 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/downloads/src/storage.rs:132` | accounts UPDATE → channel UPDATE → RBAC exclusive (production; publish) | 3 → 4 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/downloads/tests/command.rs:14` | no direct lock; permission/admission/call site (test fixture/barrier; db_command_success_refusals_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:126` | no direct lock; permission/admission/call site (test fixture/barrier; db_signed_contract_bundles_publish_for_both_channels) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:161` | no direct lock; permission/admission/call site (test fixture/barrier; db_single_connection_publication_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:196` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_drc_download_operator_joins_rbac_before_permission_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:198` | pg_advisory_xact_lock: 71010 (test fixture/barrier; postgres_drc_download_operator_joins_rbac_before_permission_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:206` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_drc_download_operator_joins_rbac_before_permission_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:211` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_drc_download_operator_joins_rbac_before_permission_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/common/mod.rs:16` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/publication.rs:638` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_drc_publication_retains_rbac_through_history_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/publication.rs:657` | LOCK TABLE: client_releases (test fixture/barrier; postgres_drc_publication_retains_rbac_through_history_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/publication.rs:697` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_drc_publication_retains_rbac_through_history_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:32` | FOR UPDATE: accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:300` | FOR UPDATE: accounts (test fixture/barrier; audit05_04_replay_waits_for_account_before_attempt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:388` | FOR UPDATE: native_recovery_handles (test fixture/barrier; audit05_05_handle_replacement_rechecks_expiry_after_lock_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:589` | FOR UPDATE: native_scope_assignments (test fixture/barrier; wave2_03_report_expires_while_assignment_locked) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:679` | FOR UPDATE: accounts (test fixture/barrier; wave2_01_unrelated_account_lock_does_not_block_recovery) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:747` | FOR UPDATE: native_recovery_handles (test fixture/barrier; wave2_02_revoke_deadline_covers_row_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:6` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:266` | no direct lock; permission/admission/call site (production; prepare_on) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:594` | no direct lock; permission/admission/call site (production; select) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:667` | no direct lock; permission/admission/call site (production; candidates) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:692` | no direct lock; permission/admission/call site (production; revalidate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:747` | FOR SHARE: game_channels (production; current) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:765` | no direct lock; permission/admission/call site (production; current) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant_tests.rs:114` | no direct lock; permission/admission/call site (test fixture/barrier; claims_are_exact_in_contract_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant_tests.rs:136` | no direct lock; permission/admission/call site (test fixture/barrier; claims_fail_closed_on_bad_report_values_and_bounds) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant_tests.rs:156` | no direct lock; permission/admission/call site (test fixture/barrier; claims_fail_closed_on_bad_report_values_and_bounds) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/http.rs:167` | no direct lock; permission/admission/call site (production; authenticate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/http.rs:168` | no direct lock; permission/admission/call site (production; authenticate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/mod.rs:2` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/mod.rs:24` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:6` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:128` | no direct lock; permission/admission/call site (production; advance) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:166` | FOR SHARE: device_sessions (production; issue) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:221` | FOR UPDATE: native_recovery_handles (production; replace) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:88` | no direct lock; permission/admission/call site (production; status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:445` | FOR UPDATE: accounts (production; serialize_identity) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:471` | FOR UPDATE: native_recovery_handles (production; authenticate) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:485` | FOR SHARE: accounts (production; authenticate) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:506` | FOR SHARE: device_sessions, accounts (production; authenticate) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:542` | FOR SHARE: native_play_generations (production; authenticate) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:670` | FOR SHARE: native_assignment_epoch_locks (production; recover_inner) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:678` | FOR UPDATE: native_recoveries (production; recover_inner) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery_tests.rs:320` | FOR UPDATE: accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery_tests.rs:505` | FOR UPDATE: accounts (test fixture/barrier; audit05_04_recovery_waits_for_account_without_holding_handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery_tests.rs:571` | FOR UPDATE: native_recovery_handles (test fixture/barrier; wave2_03_handle_expiry_rechecked_after_handle_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:25` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/store.rs:40` | no direct lock; permission/admission/call site (production; redeem) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/store.rs:54` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/store.rs:70` | no direct lock; permission/admission/call site (production; redeem) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/store.rs:85` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/store.rs:96` | pg_advisory_xact_lock (production; lock) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/store.rs:174` | FOR UPDATE: native_admissions (production; admit_inner) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/admission/tests.rs:52` | FOR UPDATE: fixture_accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:72` | no direct lock; permission/admission/call site (test fixture/barrier; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:77` | no direct lock; permission/admission/call site (test fixture/barrier; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:95` | no direct lock; permission/admission/call site (test fixture/barrier; issue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:311` | no direct lock; permission/admission/call site (test fixture/barrier; wrong_service_token_has_exact_laravel_body_without_database) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:312` | no direct lock; permission/admission/call site (test fixture/barrier; wrong_service_token_has_exact_laravel_body_without_database) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:389` | no direct lock; permission/admission/call site (test fixture/barrier; db_suspended_unknown_and_ambiguous_accounts_are_refused_and_rolled_back) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:403` | no direct lock; permission/admission/call site (test fixture/barrier; db_suspended_unknown_and_ambiguous_accounts_are_refused_and_rolled_back) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:446` | no direct lock; permission/admission/call site (test fixture/barrier; db_concurrent_same_attempt_creates_one_row) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:454` | no direct lock; permission/admission/call site (test fixture/barrier; db_consumed_ticket_cannot_authorize_another_attempt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:7` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:46` | FOR SHARE: clock_timestamp, account_character_feed (production; listing) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:90` | no direct lock; permission/admission/call site (production; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:100` | FOR UPDATE: accounts (production; redeem) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:105` | FOR UPDATE: game_login_tickets (production; redeem) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:121` | FOR UPDATE: accounts (production; redeem) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:138` | no direct lock; permission/admission/call site (production; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:167` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/wire.rs:31` | no direct lock; permission/admission/call site (production; status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:197` | FOR UPDATE: accounts (production; lock_account) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:379` | FOR UPDATE: device_sessions, device_session_credentials (production; operate_once) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:486` | FOR UPDATE: accounts (production; revoke_for_owner) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:504` | FOR UPDATE: accounts (production; revoke_for_account) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:35` | FOR SHARE: oauth_native_clients (production; lock_client) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:68` | FOR UPDATE: accounts (production; verify) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:109` | no direct lock; permission/admission/call site (production; use_native) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:126` | FOR SHARE: clock_timestamp, account_character_feed (production; use_native) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:170` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:183` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:187` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_http.rs:18` | no direct lock; permission/admission/call site (production; failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_http.rs:200` | FOR UPDATE SKIP LOCKED: device_session_source_limits, clock_timestamp (production; throttle) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_tests.rs:442` | no direct lock; permission/admission/call site (test fixture/barrier; native_setup_with) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_tests.rs:769` | no direct lock; permission/admission/call site (test fixture/barrier; database_native_normal_ticket_consumes_oauth_but_device_continues) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:471` | FOR UPDATE: accounts (production; source) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:521` | FOR UPDATE: native_game_signing_trust_profiles (production; source) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:535` | FOR UPDATE: native_game_signing_trust_key_versions (production; source) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:686` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence_tests.rs:201` | no direct lock; permission/admission/call site (test fixture/barrier; evidence_guard_size_identity_and_throttle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/mod.rs:380` | no direct lock; permission/admission/call site (production; sign) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:763` | pg_advisory_xact_lock: 715110005 (test fixture/barrier; audit05_08_cross_scope_publication_is_serialized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:20` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/keys/trust.rs:104` | pg_advisory_xact_lock: 715110005 (production; publish) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/keys/trust.rs:108` | FOR UPDATE: native_game_signing_trust_profiles (production; publish) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/keys/trust.rs:189` | FOR UPDATE: native_game_signing_trust_profiles (production; revoke) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/keys/trust.rs:238` | no direct lock; permission/admission/call site (production; snapshot) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/keys/trust.rs:240` | no direct lock; permission/admission/call site (production; snapshot) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/keys/trust.rs:245` | FOR UPDATE: native_game_signing_trust_profiles (production; snapshot_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:210` | FOR SHARE: native_assignment_epoch_locks (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:218` | FOR SHARE: native_scope_assignments (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:223` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:229` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:260` | pg_advisory_xact_lock: 0, native-recovery-locator: (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:266` | FOR UPDATE: native_recovery_locators (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:375` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:381` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:388` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:413` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator.rs:416` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/gameauth/src/locator_tests.rs:188` | no direct lock; permission/admission/call site (test fixture/barrier; route_is_off_by_default_and_keeps_identities_apart) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:192` | no direct lock; permission/admission/call site (test fixture/barrier; route_is_off_by_default_and_keeps_identities_apart) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:198` | no direct lock; permission/admission/call site (test fixture/barrier; route_is_off_by_default_and_keeps_identities_apart) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:212` | no direct lock; permission/admission/call site (test fixture/barrier; route_is_off_by_default_and_keeps_identities_apart) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:215` | no direct lock; permission/admission/call site (test fixture/barrier; route_is_off_by_default_and_keeps_identities_apart) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:392` | no direct lock; permission/admission/call site (test fixture/barrier; db_locator_acceptance_ordering_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:404` | no direct lock; permission/admission/call site (test fixture/barrier; db_locator_acceptance_ordering_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:411` | no direct lock; permission/admission/call site (test fixture/barrier; db_locator_acceptance_ordering_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:76` | no direct lock; permission/admission/call site (production; safe_redirect) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:167` | FOR UPDATE: accounts (production; credentials) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:208` | no direct lock; permission/admission/call site (production; audit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:255` | FOR UPDATE: web_sessions, accounts (production; approve_inner) | 6 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:362` | FOR UPDATE: oauth_authorization_codes (production; exchange_code) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:411` | FOR UPDATE: oauth_token_families (production; refresh) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:467` | FOR UPDATE: accounts (production; revoke_account) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:506` | no direct lock; permission/admission/call site (production; verify_scoped) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:567` | FOR UPDATE: oauth_tokens, oauth_token_families (production; native_enrollment) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs:271` | no direct lock; permission/admission/call site (test fixture/barrier; pg_token_endpoint_matches_native_oauth_ticket_fixture) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:40` | no direct lock; permission/admission/call site (production; browser_routes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:68` | no direct lock; permission/admission/call site (production; response) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:160` | no direct lock; permission/admission/call site (production; texts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:188` | no direct lock; permission/admission/call site (production; show) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:216` | no direct lock; permission/admission/call site (production; show) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:272` | no direct lock; permission/admission/call site (production; consent) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:95` | no direct lock; permission/admission/call site (test fixture/barrier; code) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:393` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_account_guards_credentials_and_explicit_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:400` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_account_guards_credentials_and_explicit_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:437` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:441` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:447` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:468` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:482` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:495` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:501` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:512` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_consent_and_oauth_envelopes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:578` | no direct lock; permission/admission/call site (test fixture/barrier; disabled_oauth_denies_without_database_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:610` | no direct lock; permission/admission/call site (test fixture/barrier; consent_policy_keeps_form_origin_and_does_not_send_query_referrers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:680` | no direct lock; permission/admission/call site (test fixture/barrier; approve_post) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:697` | no direct lock; permission/admission/call site (test fixture/barrier; codes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:703` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_session_authorizes_exchanges_refreshes_and_verifies) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:706` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_session_authorizes_exchanges_refreshes_and_verifies) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:710` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_session_authorizes_exchanges_refreshes_and_verifies) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:752` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_account_without_mfa_session_steps_up_without_a_code) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:761` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_account_without_mfa_session_steps_up_without_a_code) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:781` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_account_without_mfa_session_steps_up_without_a_code) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:790` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_account_without_mfa_session_steps_up_without_a_code) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:988` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_ida_consent_rechecks_live_session_and_mfa_after_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:294` | FOR SHARE: native_assignment_epoch_locks (production; assignment_matches) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:303` | FOR UPDATE: native_scope_assignments (production; assignment_matches) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:305` | FOR SHARE: native_scope_assignments (production; assignment_matches) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:326` | FOR UPDATE: native_runtime_status_reports (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:391` | no direct lock; permission/admission/call site (production; read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:395` | no direct lock; permission/admission/call site (production; read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:412` | FOR SHARE: native_assignment_epoch_locks (production; read_in) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:419` | FOR SHARE: native_scope_assignments (production; read_in) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:434` | no direct lock; permission/admission/call site (production; read_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:440` | no direct lock; permission/admission/call site (production; read_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:452` | no direct lock; permission/admission/call site (production; read_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:494` | no direct lock; permission/admission/call site (production; authorization_probe) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:562` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:572` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:579` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:599` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:219` | no direct lock; permission/admission/call site (test fixture/barrier; runtime_guard_refuses_disabled_wrong_identity_bad_json_and_size) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:408` | no direct lock; permission/admission/call site (test fixture/barrier; db_runtime_rejects_missing_stale_owner_epoch_scope_and_future) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:563` | no direct lock; permission/admission/call site (test fixture/barrier; db_runtime_readers_in_open_transactions_share_the_assignment_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:573` | no direct lock; permission/admission/call site (test fixture/barrier; db_runtime_readers_in_open_transactions_share_the_assignment_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:603` | FOR UPDATE: native_scope_assignments (test fixture/barrier; audit05_02_assignment_lock_precedes_report_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:611` | no direct lock; permission/admission/call site (test fixture/barrier; audit05_02_assignment_lock_precedes_report_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:1` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:176` | no direct lock; permission/admission/call site (production; publish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:201` | FOR UPDATE: native_assignment_epoch_locks (production; publish) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:215` | FOR UPDATE: native_scope_assignments (production; publish) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:261` | FOR SHARE: native_assignment_epoch_locks (production; allows) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:333` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:343` | no direct lock; permission/admission/call site (production; gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes_tests.rs:94` | no direct lock; permission/admission/call site (test fixture/barrier; default_off_and_wrong_purpose) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes_tests.rs:188` | no direct lock; permission/admission/call site (test fixture/barrier; db_assignment_revocation_ordering_and_admission_denial) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:12` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:50` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:89` | FOR SHARE: device_sessions, oauth_tokens, clock_timestamp (production; issue) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:140` | FOR UPDATE SKIP LOCKED (production; prune) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:155` | no direct lock; permission/admission/call site (production; validate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/tests.rs:13` | no direct lock; permission/admission/call site (test fixture/barrier; account_state_and_generations_fail_closed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/tests.rs:23` | no direct lock; permission/admission/call site (test fixture/barrier; account_state_and_generations_fail_closed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/src/lib.rs:896` | no direct lock; permission/admission/call site (test fixture/barrier; rejected_credentials_cannot_exhaust_lifetime_capacity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/tests/fixtures/go_vectors.jsonl:249` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/tests/go_e2e/native_login_test.go:40` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/tests/recover.rs:326` | no direct lock; permission/admission/call site (test fixture/barrier; recover_refusals_relay_byte_exact) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands.rs:249` | no direct lock; permission/admission/call site (production; require) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands.rs:447` | FOR UPDATE: hireling_command_sequence (production; enqueue) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands.rs:458` | no direct lock; permission/admission/call site (production; enqueue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:166` | no direct lock; permission/admission/call site (test fixture/barrier; strict_input_types_and_bounds) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:170` | no direct lock; permission/admission/call site (test fixture/barrier; fresh_authorized_preflight_and_typed_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:254` | no direct lock; permission/admission/call site (test fixture/barrier; fresh_authorized_preflight_and_typed_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:259` | no direct lock; permission/admission/call site (test fixture/barrier; fresh_authorized_preflight_and_typed_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/service.rs:131` | no direct lock; permission/admission/call site (production; history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/web.rs:61` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/tests/flow.rs:149` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_read_only_page_authentication_and_localization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/abuse.rs:81` | pg_advisory_xact_lock: 0 (production; issue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:33` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:44` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:55` | FOR UPDATE: wallet_accounts (production; check_blockers) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:442` | no direct lock; permission/admission/call site (production; failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/accounts.rs:42` | no direct lock; permission/admission/call site (production; find_by_id) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/accounts.rs:85` | pg_advisory_xact_lock: 0 (production; claim_lock) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/consents.rs:153` | FOR UPDATE: accounts (production; decide) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents_tests.rs:391` | no direct lock; permission/admission/call site (test fixture/barrier; consents_export_includes_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials.rs:148` | FOR UPDATE: accounts, web_sessions (production; authenticated) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/credentials_http.rs:52` | no direct lock; permission/admission/call site (production; active_verified_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:256` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:284` | no direct lock; permission/admission/call site (production; change) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:326` | no direct lock; permission/admission/call site (production; sudo) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:336` | no direct lock; permission/admission/call site (production; sudo) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:60` | FOR UPDATE: account_recovery_keys (production; proof) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:169` | FOR UPDATE: accounts (production; request) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:240` | FOR UPDATE: accounts (production; token) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:244` | FOR UPDATE: account_email_changes (production; token) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:355` | FOR UPDATE: accounts (production; apply_due) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:357` | FOR UPDATE: account_email_changes (production; apply_due) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:416` | no direct lock; permission/admission/call site (production; trade_allowed) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:443` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:444` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change.rs:445` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/email_change_mail.rs:98` | FOR UPDATE: account_email_change_mail, account_email_changes (production; tick) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change_tests.rs:498` | FOR UPDATE: accounts (test fixture/barrier; audit02_reversion_expires_after_waiting_for_account_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/export.rs:53` | no direct lock; permission/admission/call site (production; prepare_download) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/export_tests.rs:25` | no direct lock; permission/admission/call site (test fixture/barrier; disabled_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:42` | no direct lock; permission/admission/call site (production; authenticate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:58` | no direct lock; permission/admission/call site (production; current) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:100` | no direct lock; permission/admission/call site (production; authenticate_authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:101` | no direct lock; permission/admission/call site (production; authenticate_authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:103` | no direct lock; permission/admission/call site (production; authenticate_authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:104` | no direct lock; permission/admission/call site (production; authenticate_authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:107` | no direct lock; permission/admission/call site (production; authenticate_authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:110` | no direct lock; permission/admission/call site (production; authenticate_authorized) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:112` | no direct lock; permission/admission/call site (production; authorize_return) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:165` | no direct lock; permission/admission/call site (production; submit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:188` | no direct lock; permission/admission/call site (production; establish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:230` | no direct lock; permission/admission/call site (production; logout) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:240` | no direct lock; permission/admission/call site (production; accepted) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:308` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_source_limit_bad_input_and_missing_transport) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:317` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_login_returns_to_stored_authorize_target_once) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:321` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_login_returns_to_stored_authorize_target_once) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:417` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_paused_password_completion_refuses_reset_even_after_anonymous_get) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:473` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_paused_password_completion_refuses_reset_even_after_anonymous_get) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:23` | no direct lock; permission/admission/call site (test fixture/barrier; validation_and_redaction) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:24` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:25` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:26` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:30` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:31` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:32` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:34` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:35` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:36` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:37` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:38` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:39` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:40` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:43` | no direct lock; permission/admission/call site (test fixture/barrier; authorize_return_accepts_only_the_local_authorize_route) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:94` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa.rs:125` | no direct lock; permission/admission/call site (production; password_session) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa.rs:225` | FOR UPDATE: accounts (production; lock) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa.rs:655` | FOR UPDATE: account_mfa_totp (production; verify_totp_in_connection) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa.rs:739` | FOR UPDATE SKIP LOCKED: account_mfa_tokens (production; clean) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa.rs:763` | FOR UPDATE: accounts (production; staff_reset_verified) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa.rs:767` | FOR UPDATE: support_mfa_reset_evidence, tickets, accounts (production; staff_reset_verified) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/mfa_http.rs:125` | no direct lock; permission/admission/call site (production; redirect) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:337` | no direct lock; permission/admission/call site (production; validate_rotated_session) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:348` | no direct lock; permission/admission/call site (production; establish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:351` | no direct lock; permission/admission/call site (production; establish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:358` | no direct lock; permission/admission/call site (production; establish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:477` | no direct lock; permission/admission/call site (production; login) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:527` | no direct lock; permission/admission/call site (production; challenge) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:589` | no direct lock; permission/admission/call site (production; email) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http_tests.rs:67` | no direct lock; permission/admission/call site (test fixture/barrier; browser_flow) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http_tests.rs:69` | no direct lock; permission/admission/call site (test fixture/barrier; browser_flow) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_tests.rs:306` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_stale_generation_expiry_and_session_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_tests.rs:316` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_mfa_stale_generation_expiry_and_session_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:1` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:34` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:131` | no direct lock; permission/admission/call site (test fixture/barrier; show) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:142` | no direct lock; permission/admission/call site (test fixture/barrier; approve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:128` | no direct lock; permission/admission/call site (production; confirm_password) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:140` | no direct lock; permission/admission/call site (production; confirm_password) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:149` | no direct lock; permission/admission/call site (production; begin_registration_authorized) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:203` | no direct lock; permission/admission/call site (production; finish_registration) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:235` | no direct lock; permission/admission/call site (production; finish_authentication) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:237` | no direct lock; permission/admission/call site (production; finish_authentication) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:253` | FOR UPDATE: account_passkeys (production; finish_authorized) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:298` | no direct lock; permission/admission/call site (production; list) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:308` | no direct lock; permission/admission/call site (production; remove_authorized) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:331` | FOR UPDATE: accounts (production; eligible) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:339` | no direct lock; permission/admission/call site (production; eligible) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys.rs:391` | FOR UPDATE SKIP LOCKED: passkey_ceremonies (production; clean) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/passkeys_http.rs:266` | no direct lock; permission/admission/call site (production; begin_register) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:305` | no direct lock; permission/admission/call site (production; end_authenticate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:309` | no direct lock; permission/admission/call site (production; end_authenticate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:335` | no direct lock; permission/admission/call site (production; delete_key) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:39` | no direct lock; permission/admission/call site (test fixture/barrier; begin_registration) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:50` | no direct lock; permission/admission/call site (test fixture/barrier; remove) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:539` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_completed_passkey_proof_refuses_removed_key) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:576` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_removing_another_passkey_invalidates_earlier_authentication) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:617` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_registration_and_removal_refuse_revoked_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:622` | FOR UPDATE: accounts (test fixture/barrier; audit02_registration_and_removal_refuse_revoked_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:696` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_registration_and_removal_refuse_revoked_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:714` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_registration_and_removal_refuse_revoked_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:728` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_registration_and_removal_refuse_revoked_authorization) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:96` | FOR UPDATE: accounts (production; locked) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/recovery_key.rs:104` | no direct lock; permission/admission/call site (production; locked) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/recovery_key.rs:122` | no direct lock; permission/admission/call site (production; generate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/recovery_key.rs:147` | no direct lock; permission/admission/call site (production; confirm) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/recovery_key.rs:161` | no direct lock; permission/admission/call site (production; revoke) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/recovery_key.rs:196` | FOR UPDATE: accounts (production; recover) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/reset.rs:86` | FOR UPDATE: accounts, account_backup_emails (production; request) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/reset.rs:123` | FOR UPDATE: accounts, account_backup_emails (production; complete) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/reset.rs:127` | FOR UPDATE: account_password_resets, account_backup_emails (production; complete) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/reset_tests.rs:183` | no direct lock; permission/admission/call site (test fixture/barrier; login_started_before_reset_cannot_restore_session) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:240` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_password_proof_is_revoked_despite_anonymous_activity_refresh) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:330` | FOR UPDATE: accounts (test fixture/barrier; audit02_verification_and_session_expire_while_waiting_for_account_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:347` | FOR UPDATE: accounts (test fixture/barrier; audit02_verification_and_session_expire_while_waiting_for_account_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:352` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_verification_and_session_expire_while_waiting_for_account_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:357` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_verification_and_session_expire_while_waiting_for_account_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:359` | no direct lock; permission/admission/call site (test fixture/barrier; audit02_verification_and_session_expire_while_waiting_for_account_lock) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:438` | FOR UPDATE: accounts (test fixture/barrier; audit02_mfa_challenge_expires_during_account_lock_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations.rs:166` | no direct lock; permission/admission/call site (production; decode) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:82` | no direct lock; permission/admission/call site (test fixture/barrier; security_revocation_export_wrong_identity_refused) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_wizard.rs:179` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_wizard_tests.rs:96` | no direct lock; permission/admission/call site (test fixture/barrier; route_refusal_and_bad_locale) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:58` | no direct lock; permission/admission/call site (production; active_verified_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:154` | no direct lock; permission/admission/call site (production; fmt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:155` | FOR UPDATE: accounts (production; authorize) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:160` | FOR UPDATE: web_sessions (production; authorize) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:166` | no direct lock; permission/admission/call site (production; list) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:177` | no direct lock; permission/admission/call site (production; revoke) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:206` | no direct lock; permission/admission/call site (production; record_login) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions.rs:270` | no direct lock; permission/admission/call site (production; active_verified) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/sessions_http.rs:398` | no direct lock; permission/admission/call site (production; complete_login) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions_http_tests.rs:145` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_login_observer_commits_before_cookie_and_denies_enqueue_failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:1` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:23` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:40` | no direct lock; permission/admission/call site (production; fmt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:131` | no direct lock; permission/admission/call site (production; cancelled) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:154` | no direct lock; permission/admission/call site (production; revoke) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:228` | no direct lock; permission/admission/call site (production; request) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:259` | no direct lock; permission/admission/call site (production; cancel) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:384` | no direct lock; permission/admission/call site (production; verify_password) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination.rs:390` | FOR UPDATE: accounts (production; lock) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:136` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:139` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:155` | no direct lock; permission/admission/call site (production; ingest_receipt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:220` | no direct lock; permission/admission/call site (production; ingest_projection) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:231` | FOR UPDATE: plat3_closure_feed (production; ingest_projection) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:244` | FOR UPDATE: plat3_closure_snapshots (production; ingest_projection) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:298` | no direct lock; permission/admission/call site (production; ingest_watermark) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:303` | FOR UPDATE: plat3_closure_feed (production; ingest_watermark) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:372` | FOR UPDATE: plat3_closure_work (production; enqueue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:434` | no direct lock; permission/admission/call site (production; enqueue) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:440` | FOR UPDATE: plat3_closure_lifecycle (production; enqueue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:462` | FOR UPDATE: plat3_closure_feed (production; enqueue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:537` | FOR UPDATE: plat3_closure_work, plat3_closure_lifecycle (production; proof) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:559` | FOR UPDATE: plat3_closure_feed (production; proof) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:577` | FOR UPDATE: plat3_closure_snapshots (production; proof) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:599` | FOR UPDATE: account_character_feed (production; proof) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:603` | FOR UPDATE: account_character_snapshots (production; proof) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:621` | no direct lock; permission/admission/call site (production; reconcile) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure.rs:629` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:133` | no direct lock; permission/admission/call site (production; signature) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:135` | no direct lock; permission/admission/call site (production; signature) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:137` | no direct lock; permission/admission/call site (production; signature) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:140` | no direct lock; permission/admission/call site (production; signature) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:144` | no direct lock; permission/admission/call site (production; signature) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:153` | no direct lock; permission/admission/call site (production; response) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:139` | FOR SHARE: character_restore_locks (production; current) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:168` | FOR UPDATE: account_terminations (production; read_lifecycle) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:175` | FOR UPDATE: plat3_closure_work, plat3_closure_lifecycle (production; read_lifecycle) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:197` | FOR UPDATE: wallet_accounts (production; read_lifecycle) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:46` | no direct lock; permission/admission/call site (test fixture/barrier; current) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:56` | no direct lock; permission/admission/call site (test fixture/barrier; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:803` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_production_factory_and_guarded_http_pull_read_signed_receipt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:823` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_production_factory_and_guarded_http_pull_read_signed_receipt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:951` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_production_factory_and_guarded_http_pull_read_signed_receipt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:70` | no direct lock; permission/admission/call site (test fixture/barrier; close) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:80` | no direct lock; permission/admission/call site (test fixture/barrier; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:184` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_request_cancel_reschedule_and_finalize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:200` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_request_cancel_reschedule_and_finalize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:284` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_bad_input_and_actor_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:295` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_bad_input_and_actor_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/verify.rs:44` | FOR UPDATE: accounts (production; consume) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/identity/src/verify.rs:58` | FOR UPDATE: accounts (production; resend) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/internal-api/src/guard.rs:104` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/internal-api/src/guard.rs:238` | no direct lock; permission/admission/call site (production; check_peer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/internal-api/tests/contracts.rs:117` | no direct lock; permission/admission/call site (test fixture/barrier; denies_unknown_missing_wrong_group_and_spoofed_headers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/internal-api/tests/contracts.rs:151` | no direct lock; permission/admission/call site (test fixture/barrier; unavailable_precedes_authentication) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/store.rs:50` | FOR UPDATE: accounts (production; lock_account) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/store.rs:255` | FOR UPDATE: collection_decks (production; start) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/store.rs:320` | FOR UPDATE: collection_battles (production; turn) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:135` | no direct lock; permission/admission/call site (production; active) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:140` | pg_advisory_xact_lock: 92 (production; creation_budget) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:174` | FOR UPDATE: collection_pvp_battles (production; accept) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:240` | FOR UPDATE: collection_pvp_tournaments (production; play) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:245` | FOR UPDATE: collection_pvp_battles (production; play) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:260` | FOR UPDATE: collection_pvp_tournaments (production; play) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:324` | FOR UPDATE: collection_pvp_tournaments (production; join) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:355` | FOR UPDATE: collection_pvp_tournaments (production; start) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:416` | pg_advisory_xact_lock: 91 (production; buy_set) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:433` | FOR SHARE: collection_pvp_sets (production; buy_set) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:532` | no direct lock; permission/admission/call site (production; debit) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:466` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:527` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_store_wallet_receipt_cannot_buy_a_set) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/README.md:43` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:59` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:63` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:98` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:110` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:114` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:149` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:49` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:58` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:59` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:69` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:81` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:82` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:83` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:96` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:97` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:98` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:111` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:112` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:113` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:127` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:128` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:129` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:136` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:143` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:144` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:151` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:41` | FOR SHARE: league_feed_state (production; checked_fence) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:196` | pg_advisory_xact_lock: 94000003 (production; question) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:201` | FOR UPDATE: league_event_questions (production; question) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:242` | FOR KEY SHARE: world_status (production; registered_world) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:260` | pg_advisory_xact_lock: 94000003 (production; initialize_inner) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:263` | FOR UPDATE: league_feed_state (production; initialize_inner) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:308` | pg_advisory_xact_lock: 94000003 (production; receive_inner) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:312` | FOR UPDATE: league_feed_requests (production; receive_inner) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:423` | pg_try_advisory_lock: 94000004 (production; run_config) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:473` | pg_advisory_xact_lock: 94000003 (production; run_config) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:542` | pg_advisory_xact_lock: 94000003 (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:548` | FOR UPDATE: league_subjects (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:580` | pg_advisory_xact_lock: 94000003 (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:586` | FOR UPDATE: league_feed_state (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:588` | FOR UPDATE: league_feed_state (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:613` | pg_advisory_xact_lock: 94000003 (production; install) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:616` | FOR UPDATE: league_feed_state (production; install) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/runtime.rs:662` | pg_advisory_xact_lock: 94000003 (production; run) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/store.rs:91` | pg_advisory_xact_lock: 94000003 (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/store.rs:118` | FOR UPDATE: league_subjects (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/store.rs:207` | pg_advisory_xact_lock_shared: 94000003 (production; read_inner) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/store.rs:269` | pg_advisory_xact_lock: 94000003 (production; expire) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/league/tests.rs:1284` | pg_advisory_xact_lock: 94000003 (test fixture/barrier; postgres_league_lock_contention_has_a_deadline_and_releases_pool) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:72` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:75` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:198` | no direct lock; permission/admission/call site (production; start) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:205` | no direct lock; permission/admission/call site (production; start) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:206` | no direct lock; permission/admission/call site (production; start) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:276` | no direct lock; permission/admission/call site (test fixture/barrier; game_json_without_headers_is_admitted_only_for_verified_pinned_peer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/mod.rs:217` | no direct lock; permission/admission/call site (production; debit) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/repository.rs:74` | FOR SHARE: pass_seasons (production; lock) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/repository.rs:82` | FOR UPDATE: pass_progress (production; lock) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/tests.rs:459` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/tests.rs:694` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_store_wallet_receipt_cannot_buy_a_pass) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/web.rs:200` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/web.rs:236` | no direct lock; permission/admission/call site (production; buy) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/web.rs:253` | no direct lock; permission/admission/call site (production; claim) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/http.rs:156` | no direct lock; permission/admission/call site (production; account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/mod.rs:1` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/store.rs:154` | FOR UPDATE: accounts (production; answer) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/store.rs:168` | FOR UPDATE: puzzles_streaks (production; answer) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/tests.rs:710` | FOR UPDATE: puzzles_streaks (test fixture/barrier; postgres_answer_holds_account_lock_while_waiting_for_streak) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:265` | no direct lock; permission/admission/call site (production; account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:267` | no direct lock; permission/admission/call site (production; account_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:289` | no direct lock; permission/admission/call site (production; preferences) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:304` | no direct lock; permission/admission/call site (production; enqueue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:356` | no direct lock; permission/admission/call site (production; unsubscribe) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:26` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:72` | no direct lock; permission/admission/call site (production; refusal) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:74` | no direct lock; permission/admission/call site (production; refusal) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:100` | no direct lock; permission/admission/call site (production; payer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:102` | no direct lock; permission/admission/call site (production; payer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:108` | no direct lock; permission/admission/call site (production; payer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:27` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:140` | no direct lock; permission/admission/call site (production; fmt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:145` | no direct lock; permission/admission/call site (production; state_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:154` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:274` | no direct lock; permission/admission/call site (production; restriction) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:374` | pg_advisory_xact_lock: 90105 (production; catalogue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:378` | FOR UPDATE: coin_purchase_quotes, coin_purchase_orders (production; catalogue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:497` | no direct lock; permission/admission/call site (production; find_order) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:545` | pg_advisory_xact_lock: 90105 (production; insert_order) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:561` | FOR UPDATE: coin_purchase_quotes (production; insert_order) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:698` | no direct lock; permission/admission/call site (production; webhook) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:710` | no direct lock; permission/admission/call site (production; webhook) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:717` | no direct lock; permission/admission/call site (production; webhook_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:731` | no direct lock; permission/admission/call site (production; verified) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:800` | FOR UPDATE: payment_intents (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:805` | FOR UPDATE: coin_purchase_orders (production; apply) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:896` | no direct lock; permission/admission/call site (production; settle) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:1003` | FOR UPDATE: wallet_accounts (production; reverse) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:1010` | no direct lock; permission/admission/call site (production; reverse) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:1136` | no direct lock; permission/admission/call site (production; purchase_hold) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/core.rs:43` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/core.rs:49` | no direct lock; permission/admission/call site (production; state_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/core.rs:54` | no direct lock; permission/admission/call site (production; refund_state_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/provider_webhook.rs:340` | pg_advisory_xact_lock: 0, payment_provider_webhook: (production; record) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/provider_webhook.rs:423` | no direct lock; permission/admission/call site (production; reverse) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/provider_webhook.rs:460` | no direct lock; permission/admission/call site (production; respond) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/storage.rs:149` | no direct lock; permission/admission/call site (production; intent) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:150` | no direct lock; permission/admission/call site (production; intent) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:156` | FOR SHARE: payment_intents (production; intent_in) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:159` | no direct lock; permission/admission/call site (production; intent_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:187` | FOR UPDATE: payment_intents (production; webhook) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:239` | no direct lock; permission/admission/call site (production; webhook) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:241` | no direct lock; permission/admission/call site (production; webhook) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/payments.rs:15` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:39` | no direct lock; permission/admission/call site (test fixture/barrier; payer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:59` | pg_advisory_xact_lock: 90101 (test fixture/barrier; credit_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:89` | pg_advisory_xact_lock (test fixture/barrier; reverse_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:513` | no direct lock; permission/admission/call site (test fixture/barrier; signature_and_timestamp_are_required) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:519` | no direct lock; permission/admission/call site (test fixture/barrier; signature_and_timestamp_are_required) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/README.md:49` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:162` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:164` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:166` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:172` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:178` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/client_fixtures_tests.rs:232` | no direct lock; permission/admission/call site (test fixture/barrier; bearer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:71` | no direct lock; permission/admission/call site (production; bearer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:73` | no direct lock; permission/admission/call site (production; bearer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:75` | no direct lock; permission/admission/call site (production; bearer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:77` | no direct lock; permission/admission/call site (production; bearer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:83` | no direct lock; permission/admission/call site (production; failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:99` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:153` | no direct lock; permission/admission/call site (production; characters) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:172` | no direct lock; permission/admission/call site (production; characters) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:97` | FOR UPDATE: test_oauth (test fixture/barrier; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:102` | no direct lock; permission/admission/call site (test fixture/barrier; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:107` | no direct lock; permission/admission/call site (test fixture/barrier; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:212` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_ticket_consumes_once_and_stores_only_digest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:235` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_suspended_account_and_failed_insert_roll_back_consumption) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:300` | no direct lock; permission/admission/call site (test fixture/barrier; bad_input_authentication_and_default_flag) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:312` | no direct lock; permission/admission/call site (test fixture/barrier; bad_input_authentication_and_default_flag) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:419` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_character_golden_owner_only_and_stale_refusal) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/tests/account_feed.rs:62` | no direct lock; permission/admission/call site (test fixture/barrier; success_refusal_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/tests/account_feed.rs:87` | no direct lock; permission/admission/call site (test fixture/barrier; success_refusal_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/tests/account_feed.rs:98` | no direct lock; permission/admission/call site (test fixture/barrier; success_refusal_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority.rs:86` | pg_advisory_xact_lock_shared: 22005160000 (production; shared_fences) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:90` | FOR SHARE: character_restore_locks (production; shared_fences) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:116` | FOR SHARE: accounts (production; account) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:130` | FOR SHARE: account_character_snapshots (production; account_epoch) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:146` | FOR SHARE: account_character_feed (production; live_feed) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:227` | FOR SHARE: character_commands (production; initial_cut) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:236` | FOR SHARE: accepted_at, pd_ownership_baselines (production; initial_cut) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:257` | FOR SHARE: character_commands (production; ownership) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority.rs:286` | FOR SHARE: account_characters, account_character_snapshots (production; ownership) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority/reads.rs:102` | FOR SHARE: pd_streams (production; cut) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority/reads.rs:371` | no direct lock; permission/admission/call site (production; prove) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/authority/tests.rs:492` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pending_transfer_then_committed_transfer_moves_every_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:498` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pending_transfer_then_committed_transfer_moves_every_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:505` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pending_transfer_then_committed_transfer_moves_every_read) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:992` | for update (test fixture/barrier; postgres_global_feed_invalidation_refuses_admission_and_ownership) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1066` | pg_advisory_xact_lock: 326787645 (test fixture/barrier; postgres_login_baseline_batches_are_complete_atomic_and_allow_other_accounts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1069` | pg_advisory_xact_lock: 326787645 (test fixture/barrier; postgres_login_baseline_batches_are_complete_atomic_and_allow_other_accounts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1713` | FOR NO KEY UPDATE: accounts (test fixture/barrier; postgres_drc_account_precedes_issuance_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1742` | pg_try_advisory_xact_lock: 22005160000 (test fixture/barrier; postgres_drc_account_precedes_issuance_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/README.md:44` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:26` | no direct lock; permission/admission/call site (production; denied) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:94` | no direct lock; permission/admission/call site (production; load) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:129` | no direct lock; permission/admission/call site (production; load) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:235` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:238` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:253` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/README.md:3` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/http.rs:199` | no direct lock; permission/admission/call site (test fixture/barrier; endpoints_default_off_and_deny_missing_session) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/storage.rs:258` | FOR SHARE: pd_t78_streams (production; read) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/storage.rs:331` | FOR UPDATE: pd_t78_streams (production; lock_stream) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/tests.rs:219` | FOR SHARE: test_authority (test fixture/barrier; stream) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/tests.rs:624` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_owner_page_success_denial_and_bad_input) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/hunts.rs:43` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/hunts_store.rs:46` | FOR UPDATE: pd_hunt_sessions (production; project) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:87` | no direct lock; permission/admission/call site (production; stream_key) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:109` | no direct lock; permission/admission/call site (production; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:207` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:215` | FOR UPDATE: pd_t76_streams (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:390` | no direct lock; permission/admission/call site (production; inventory) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:467` | no direct lock; permission/admission/call site (production; history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/tests.rs:302` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/tests.rs:778` | no direct lock; permission/admission/call site (test fixture/barrier; cookie) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/web.rs:182` | no direct lock; permission/admission/call site (production; items) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/web.rs:271` | no direct lock; permission/admission/call site (production; history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:359` | no direct lock; permission/admission/call site (test fixture/barrier; flag_off_wrong_identity_wrong_source_and_bad_body) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:371` | no direct lock; permission/admission/call site (test fixture/barrier; flag_off_wrong_identity_wrong_source_and_bad_body) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:385` | no direct lock; permission/admission/call site (test fixture/barrier; flag_off_wrong_identity_wrong_source_and_bad_body) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:909` | no direct lock; permission/admission/call site (test fixture/barrier; progress_budget_is_600_shared_by_its_routes_and_precedes_transport) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:946` | no direct lock; permission/admission/call site (test fixture/barrier; items_over_budget_peer_gets_429_before_size_checks) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins.rs:286` | FOR UPDATE: pd_game_logins (production; apply_locked_cut) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/logins.rs:325` | no direct lock; permission/admission/call site (production; history) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/logins/tests.rs:158` | FOR UPDATE: accounts (test fixture/barrier; current) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:532` | no direct lock; permission/admission/call site (test fixture/barrier; nwave_expired_session_payload_preserves_recent_logout_activity) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:554` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_authorized_epoch_rebuild_reuses_retained_occurrence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:561` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_authorized_epoch_rebuild_reuses_retained_occurrence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/README.md:25` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/retention.rs:11` | pg_advisory_xact_lock: 81005230117 (production; purge) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/places/store.rs:223` | pg_advisory_xact_lock: 81005230117 (production; set_preference) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/places/store.rs:248` | pg_advisory_xact_lock: 81005230117 (production; ingest) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/places/store.rs:294` | pg_advisory_xact_lock: 81005230117 (production; daily) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/places/tests.rs:100` | no direct lock; permission/admission/call site (test fixture/barrier; account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:136` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_thresholds_delay_rates_and_immutable_retry) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:190` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_owner_optout_recompute_and_replay_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:222` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_owner_optout_recompute_and_replay_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:241` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_unknown_evidence_bad_input_and_flags_deny) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:255` | no direct lock; permission/admission/call site (production; check) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:306` | no direct lock; permission/admission/call site (production; set_selection) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:310` | no direct lock; permission/admission/call site (production; set_selection) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:327` | no direct lock; permission/admission/call site (production; clear_selection) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:350` | no direct lock; permission/admission/call site (production; estimate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:358` | no direct lock; permission/admission/call site (production; estimate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:406` | no direct lock; permission/admission/call site (production; estimate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:408` | FOR SHARE: updated_at, pd_price_overrides (production; estimate) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/npc.rs:38` | no direct lock; permission/admission/call site (production; select) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/mod.rs:200` | no direct lock; permission/admission/call site (production; validate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/mod.rs:224` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:38` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:42` | FOR UPDATE: pd_progress_streams (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:111` | FOR UPDATE: pd_progress_streams (production; watermark) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:136` | no direct lock; permission/admission/call site (production; watermark) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:193` | no direct lock; permission/admission/call site (production; read_observation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:135` | no direct lock; permission/admission/call site (production; schedule) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:161` | FOR UPDATE: pd_scoped_reminders (production; dispatch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:192` | no direct lock; permission/admission/call site (production; cancel) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:246` | no direct lock; permission/admission/call site (production; message) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:284` | no direct lock; permission/admission/call site (test fixture/barrier; dashboard_http_success_disabled_anonymous_staff_and_bad_language) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:131` | no direct lock; permission/admission/call site (production; snapshot) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:139` | no direct lock; permission/admission/call site (production; snapshot) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:274` | no direct lock; permission/admission/call site (production; note) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:289` | no direct lock; permission/admission/call site (production; save_note) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:315` | no direct lock; permission/admission/call site (production; email_preference) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:334` | no direct lock; permission/admission/call site (production; export_notes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:366` | FOR SHARE: accounts (production; email_week) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:387` | FOR UPDATE: pd_report_preferences (production; email_week) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:395` | FOR UPDATE: account_notification_prefs (production; email_week) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports/README.md:17` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports/README.md:64` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports_tests.rs:63` | no direct lock; permission/admission/call site (test fixture/barrier; snapshot) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:22` | FOR UPDATE SKIP LOCKED: pd_g9_records (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:23` | FOR UPDATE SKIP LOCKED: pd_records (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:25` | FOR UPDATE SKIP LOCKED: pd_t76_streams (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:27` | FOR UPDATE: pd_t76_records, pd_t76_streams (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:29` | FOR UPDATE SKIP LOCKED: pd_t76_events (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:31` | FOR UPDATE: pd_t78_records (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:37` | FOR UPDATE SKIP LOCKED: pd_streams, pd_records, candidates (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:39` | FOR UPDATE: pd_records, pd_streams (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:41` | FOR UPDATE SKIP LOCKED: pd_deleted_scopes (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:43` | FOR UPDATE SKIP LOCKED: pd_conflicts (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:47` | FOR UPDATE SKIP LOCKED: pd_t76_streams, pd_t76_records, s.updated_at, candidates (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:49` | FOR UPDATE: pd_t76_records, pd_t76_streams (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:51` | FOR UPDATE: pd_t76_events, pd_t76_streams, s.updated_at (production; expire_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:68` | FOR UPDATE SKIP LOCKED: pd_erasure_jobs (production; erase_lifecycle_batch) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:178` | no direct lock; permission/admission/call site (production; handle) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:226` | pg_advisory_xact_lock: 73 (production; ingest) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:242` | pg_advisory_xact_lock: 729 (production; ingest) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:247` | FOR UPDATE: pd_g9_streams (production; ingest) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:379` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:388` | pg_advisory_xact_lock: 729 (production; enqueue) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:399` | FOR UPDATE: pd_g9_commands (production; enqueue) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/sessions.rs:454` | FOR UPDATE: pd_g9_commands (production; exchange) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/store.rs:335` | pg_advisory_xact_lock: 73 (production; lock_account) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/store.rs:343` | pg_advisory_xact_lock: 72 (production; lock) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/store.rs:353` | pg_advisory_xact_lock_shared: 72 (production; shared_authority) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/store.rs:362` | FOR UPDATE: pd_streams (production; stream) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/store.rs:443` | pg_advisory_xact_lock: 74 (production; ingest) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/playerdata/src/store/http.rs:113` | no direct lock; permission/admission/call site (production; response) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/store/tests.rs:546` | no direct lock; permission/admission/call site (test fixture/barrier; http_disabled_peer_bad_input_and_no_store) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/README.md:46` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online.rs:54` | no direct lock; permission/admission/call site (production; permits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online.rs:62` | no direct lock; permission/admission/call site (production; status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:22` | no direct lock; permission/admission/call site (production; ingest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:27` | FOR UPDATE: public_projection_state (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:154` | FOR UPDATE: public_projection_state (production; purge) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:172` | FOR SHARE: public_projection_state (production; fresh) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:67` | no direct lock; permission/admission/call site (production; owner_online_ranking) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:134` | no direct lock; permission/admission/call site (production; owner_online_ranking) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:135` | no direct lock; permission/admission/call site (production; owner_online_ranking) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:139` | no direct lock; permission/admission/call site (production; owner_online_ranking) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:146` | no direct lock; permission/admission/call site (production; owner_online_ranking) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch.rs:105` | pg_advisory_xact_lock: 70 (production; subscribe) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/stats/src/watch.rs:171` | FOR UPDATE: stats_watches (production; dispatch) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/stats/src/watch/README.md:8` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/README.md:28` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:1` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:80` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:82` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:90` | no direct lock; permission/admission/call site (production; owner) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/tests.rs:281` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_authentication_input_and_feature_gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/tests.rs:303` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_authentication_input_and_feature_gate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/worker.rs:45` | pg_advisory_xact_lock: 65001 (production; run) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/stats/src/worker.rs:53` | FOR SHARE: public_projection_state (production; run) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/stats/src/worker.rs:172` | pg_advisory_xact_lock: 65001 (production; retention) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/stats/src/world/kills.rs:20` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:96` | no direct lock; permission/admission/call site (production; parse) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:154` | FOR UPDATE: public_projection_state (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:168` | FOR UPDATE: stats_world_kills_v2 (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:138` | no direct lock; permission/admission/call site (test fixture/barrier; characters) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:402` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_owner_ordering_coverage_and_privacy_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:407` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_owner_ordering_coverage_and_privacy_denials) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:838` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_acknowledgements_authority_and_family_switches) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:904` | no direct lock; permission/admission/call site (test fixture/barrier; characters) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:950` | no direct lock; permission/admission/call site (test fixture/barrier; coverage) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/world_kills.rs:42` | no direct lock; permission/admission/call site (test fixture/barrier; world_kills_refuses_disabled_and_invalid_wire) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog.rs:213` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/catalog.rs:239` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; import) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/catalog.rs:264` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; import) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/catalog.rs:283` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; activate) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/catalog.rs:286` | FOR UPDATE: store_active_catalog (production; activate) | 7 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/catalog.rs:300` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; activate) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/catalog.rs:306` | accounts UPDATE → RBAC exclusive (operator permission helper) (production; activate) | 3 → 5 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/catalog_tests.rs:509` | FOR SHARE: accounts (test fixture/barrier; account_state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:521` | FOR UPDATE: accounts (test fixture/barrier; permitted) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:534` | FOR SHARE: store_active_catalog (test fixture/barrier; permitted) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:39` | no direct lock; permission/admission/call site (production; from) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:48` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:161` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; generate_staff) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:166` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; generate_staff) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:170` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; generate_staff) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:176` | no direct lock; permission/admission/call site (production; generate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:183` | no direct lock; permission/admission/call site (production; generate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:187` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; generate) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:190` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; generate_in) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:263` | no direct lock; permission/admission/call site (production; redeem_player) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:277` | no direct lock; permission/admission/call site (production; redeem_impl) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:281` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; redeem_impl) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:296` | pg_advisory_xact_lock: 55 (production; redeem_in) | 2 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:300` | FOR UPDATE: gift_codes, code_batches (production; redeem_in) | 2 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:325` | pg_advisory_xact_lock: 56 (production; redeem_in) | 2 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:333` | FOR UPDATE: accounts (production; redeem_in) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:381` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; redeem_in) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes.rs:394` | ordered actor account UPDATE → RBAC exclusive → session/MFA UPDATE (staff generator) (production; redeem_in) | 3 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/store/src/codes_tests.rs:19` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:158` | no direct lock; permission/admission/call site (test fixture/barrier; account) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:159` | no direct lock; permission/admission/call site (test fixture/barrier; staff) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:319` | no direct lock; permission/admission/call site (test fixture/barrier; db_expired_future_bad_input_and_disabled) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:441` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_adjust_in_commit_rollback_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:449` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_adjust_in_commit_rollback_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:458` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_adjust_in_commit_rollback_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:486` | no direct lock; permission/admission/call site (test fixture/barrier; db_premium_mutate_in_commit_rollback_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:490` | no direct lock; permission/admission/call site (test fixture/barrier; db_premium_mutate_in_commit_rollback_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:493` | no direct lock; permission/admission/call site (test fixture/barrier; db_premium_mutate_in_commit_rollback_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:610` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_browser_premium_redemption_one_connection_and_value_restrictions) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:708` | no direct lock; permission/admission/call site (test fixture/barrier; browser_redemption_locks_code_before_account_and_rechecks_status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:718` | FOR UPDATE: gift_codes (test fixture/barrier; browser_redemption_locks_code_before_account_and_rechecks_status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:741` | FOR UPDATE NOWAIT: accounts (test fixture/barrier; browser_redemption_locks_code_before_account_and_rechecks_status) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:768` | pg_advisory_xact_lock: 55 (test fixture/barrier; postgres_drc_gift_request_has_shared_budget) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:49` | no direct lock; permission/admission/call site (production; request) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:90` | pg_advisory_xact_lock: 29,1 (production; page) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:166` | pg_advisory_xact_lock: 29,1 (production; receipt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:169` | FOR UPDATE: store_operations (production; receipt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:198` | FOR UPDATE: store_fulfilment_units (production; receipt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:229` | no direct lock; permission/admission/call site (production; receipt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:246` | FOR UPDATE: store_delivery_fence (production; reopen_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:254` | pg_advisory_xact_lock: 29,1 (production; reopen_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:257` | FOR UPDATE: store_operations, store_receipts (production; reopen_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:279` | FOR UPDATE: character_restore_locks (production; lock_restore) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery.rs:281` | FOR SHARE: character_restore_locks (production; lock_restore) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/delivery_http.rs:186` | no direct lock; permission/admission/call site (production; failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:423` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_item_delivery_creates_one_stable_claim) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:449` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_restore_reopens_retains_history_and_fences_old_receipts) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:491` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_restore_carries_unresolved_and_preserves_current_exact_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:501` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_restore_carries_unresolved_and_preserves_current_exact_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:510` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_restore_carries_unresolved_and_preserves_current_exact_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:616` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_peer_transport_canonical_and_source_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:628` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_peer_transport_canonical_and_source_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:664` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_peer_transport_canonical_and_source_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:753` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_restore_lock_serializes_receipts_and_rollback_keeps_evidence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:1002` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_http_read_receipt_and_reconcile_round_trip) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:1034` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_restore_selection_is_bounded_and_atomic) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:1062` | no direct lock; permission/admission/call site (test fixture/barrier; non_rfc_v7_restore_event_never_changes_store_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/loyalty.rs:61` | FOR UPDATE: premium_authority (production; snapshot_attempt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/loyalty_stream.rs:141` | FOR UPDATE: loyalty_stream_head (production; read_attempt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/loyalty_stream.rs:191` | FOR KEY SHARE: accounts (production; read_attempt) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/loyalty_stream.rs:212` | FOR KEY SHARE: premium_authority (production; read_attempt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/loyalty_stream_http.rs:39` | FOR SHARE: accounts (production; state) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:41` | FOR UPDATE: premium_authority (production; lock) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:82` | pg_advisory_xact_lock: 27 (production; mutate) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:138` | no direct lock; permission/admission/call site (production; mutate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:150` | no direct lock; permission/admission/call site (production; mutate_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:154` | no direct lock; permission/admission/call site (production; mutate_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:162` | FOR UPDATE: referral_campaign_rewards, referral_campaign_settings (production; grant_referral_in) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:169` | no direct lock; permission/admission/call site (production; grant_referral_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:181` | no direct lock; permission/admission/call site (production; grant_referral_in) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium/repository.rs:205` | pg_advisory_xact_lock: 27 (production; authorized_in) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/premium_profile_b.rs:192` | FOR UPDATE: profile_b_authority (production; read) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium_profile_b_tests.rs:630` | no direct lock; permission/admission/call site (test fixture/barrier; db_premium_lifecycle_changes_publish_hints_in_their_transaction) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase.rs:19` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:25` | FOR UPDATE, FOR SHARE (production; module/SQL) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:26` | FOR SHARE (production; module/SQL) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:34` | FOR SHARE: accounts (production; buyable) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:73` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:77` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_http.rs:200` | no direct lock; permission/admission/call site (production; failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:38` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:67` | no direct lock; permission/admission/call site (production; source) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:151` | pg_advisory_xact_lock: 29 (production; purchase_attempt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:179` | FOR SHARE: store_active_catalog (production; purchase_attempt) | 7 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:201` | no direct lock; permission/admission/call site (production; purchase_attempt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:241` | pg_advisory_xact_lock: 29,1 (production; purchase_attempt) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:255` | no direct lock; permission/admission/call site (production; purchase_attempt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:282` | no direct lock; permission/admission/call site (production; purchase_attempt) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:416` | no direct lock; permission/admission/call site (production; read) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_repository.rs:457` | no direct lock; permission/admission/call site (production; read) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/store/src/purchase_tests.rs:280` | FOR UPDATE: purchase_test_accounts (test fixture/barrier; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:322` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1014` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_conflicts_and_unknown_state_deny_without_stored_answer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1028` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_conflicts_and_unknown_state_deny_without_stored_answer) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1099` | no direct lock; permission/admission/call site (test fixture/barrier; db_http_guard_transport_and_source_failures_are_empty) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1106` | no direct lock; permission/admission/call site (test fixture/barrier; db_http_guard_transport_and_source_failures_are_empty) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1140` | no direct lock; permission/admission/call site (test fixture/barrier; db_http_guard_transport_and_source_failures_are_empty) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1217` | no direct lock; permission/admission/call site (test fixture/barrier; db_http_guard_transport_and_source_failures_are_empty) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1265` | no direct lock; permission/admission/call site (test fixture/barrier; db_peer_and_account_limits_have_contract_precedence_and_shared_scope) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1534` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_upgrade_preserves_existing_rows_and_uuid_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1551` | no direct lock; permission/admission/call site (test fixture/barrier; db_wallet_upgrade_preserves_existing_rows_and_uuid_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1690` | FOR UPDATE: accounts (test fixture/barrier; postgres_transfer_prepares_complete_identity_set_before_wallets) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1717` | FOR UPDATE NOWAIT: wallet_accounts (test fixture/barrier; postgres_transfer_prepares_complete_identity_set_before_wallets) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web.rs:40` | no direct lock; permission/admission/call site (production; from) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web.rs:125` | no direct lock; permission/admission/call site (production; fmt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_README.md:18` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:225` | no direct lock; permission/admission/call site (production; safe_return) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:237` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:250` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:251` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:252` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:275` | no direct lock; permission/admission/call site (production; active) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:12` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:17` | no direct lock; permission/admission/call site (production; state_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:23` | no direct lock; permission/admission/call site (production; state_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:30` | FOR SHARE: accounts (production; refund_state_in) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:41` | no direct lock; permission/admission/call site (production; state) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_tests.rs:74` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:180` | no direct lock; permission/admission/call site (production; history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:194` | FOR UPDATE: support_appeal_workflows (production; change) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:225` | no direct lock; permission/admission/call site (production; account_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:236` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:254` | no direct lock; permission/admission/call site (production; account_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:309` | no direct lock; permission/admission/call site (production; detail) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:349` | no direct lock; permission/admission/call site (production; change) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals/tests.rs:203` | no direct lock; permission/admission/call site (test fixture/barrier; account_access) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments.rs:88` | no direct lock; permission/admission/call site (production; process) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments.rs:105` | no direct lock; permission/admission/call site (production; append_private) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments.rs:167` | no direct lock; permission/admission/call site (production; append) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments/tests.rs:70` | no direct lock; permission/admission/call site (test fixture/barrier; process) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments/tests.rs:89` | no direct lock; permission/admission/call site (test fixture/barrier; append_private) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:159` | FOR UPDATE: accounts (production; account) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:284` | FOR UPDATE: accounts (production; staff_apply) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:296` | FOR UPDATE: player_reports (production; staff_apply) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:25` | FOR SHARE: accounts (production; require_game_access) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:110` | no direct lock; permission/admission/call site (production; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:144` | no direct lock; permission/admission/call site (production; issue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:182` | no direct lock; permission/admission/call site (production; verify) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:210` | no direct lock; permission/admission/call site (production; from_lookup) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:316` | no direct lock; permission/admission/call site (production; digest) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:337` | FOR UPDATE: support_enforcement_counter (production; record) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:707` | FOR UPDATE: support_enforcement_commands (production; ingest) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement_tests.rs:367` | no direct lock; permission/admission/call site (test fixture/barrier; refusal_wrong_identity_or_scope_is_401) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement_tests.rs:377` | no direct lock; permission/admission/call site (test fixture/barrier; refusal_wrong_identity_or_scope_is_401) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement_tests.rs:403` | no direct lock; permission/admission/call site (test fixture/barrier; refusal_budget_precedes_transport_and_counts_malformed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export.rs:143` | no direct lock; permission/admission/call site (production; decode) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:86` | no direct lock; permission/admission/call site (test fixture/barrier; sanction_export_wrong_identity_refused) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:209` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:743` | FOR UPDATE: support_sanction_export_counter (test fixture/barrier; sanction_export_temporary_namelock_revokes_on_expiry) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:871` | LOCK TABLE: support_sanction_revisions (test fixture/barrier; postgres_expiry_candidates_bound_work_with_permanent_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:912` | LOCK TABLE: support_sanction_revisions (test fixture/barrier; postgres_expiry_candidates_bound_work_with_permanent_history) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/mod.rs:122` | no direct lock; permission/admission/call site (production; valid_id) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:13` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:19` | FOR SHARE: accounts (production; state_in) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:77` | no direct lock; permission/admission/call site (production; staff_preflight) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:80` | no direct lock; permission/admission/call site (production; staff_preflight) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:97` | FOR SHARE: accounts (production; staff) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:104` | no direct lock; permission/admission/call site (production; staff) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:107` | no direct lock; permission/admission/call site (production; staff) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:124` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:125` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:130` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:135` | FOR UPDATE: accounts (production; create) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:209` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:210` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:215` | FOR UPDATE: sanctions (production; transition) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:224` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:229` | FOR UPDATE: accounts (production; transition) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:237` | FOR UPDATE: sanctions (production; transition) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:308` | no direct lock; permission/admission/call site (production; record) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:349` | FOR UPDATE: accounts (production; lock_revision) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:361` | FOR UPDATE: support_sanction_counters (production; lock_revision) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:552` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:556` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:559` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:561` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:568` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:569` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:570` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; staff_apply) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:578` | FOR UPDATE: accounts (production; staff_apply) | 3 | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:586` | FOR UPDATE: effective_at, expires_at, sanctions (production; staff_apply) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/store.rs:589` | FOR UPDATE: support_appeal_workflows (production; staff_apply) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/support/src/sanctions/tests.rs:79` | no direct lock; permission/admission/call site (test fixture/barrier; new) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:241` | no direct lock; permission/admission/call site (test fixture/barrier; db_warning_restriction_and_suspension_ticket_lifecycle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:332` | no direct lock; permission/admission/call site (test fixture/barrier; db_bad_input_mfa_permission_and_disabled_feature_have_no_effect) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:424` | no direct lock; permission/admission/call site (test fixture/barrier; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:426` | no direct lock; permission/admission/call site (test fixture/barrier; redeem) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:444` | no direct lock; permission/admission/call site (test fixture/barrier; issue) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:548` | FOR UPDATE: accounts (test fixture/barrier; db_access_transaction_serializes_suspension_and_rolls_back_ticket_failure) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:618` | no direct lock; permission/admission/call site (test fixture/barrier; db_http_admission_success_then_suspension_denies_fresh_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:637` | no direct lock; permission/admission/call site (test fixture/barrier; db_http_admission_success_then_suspension_denies_fresh_and_replay) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:1080` | FOR UPDATE: accounts (test fixture/barrier; postgres_legacy_sanctions_lock_accounts_before_rbac) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:1105` | pg_try_advisory_xact_lock: 71010 (test fixture/barrier; postgres_legacy_sanctions_lock_accounts_before_rbac) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:4` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:88` | FOR UPDATE: accounts (production; lock_account) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:213` | no direct lock; permission/admission/call site (production; change) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:220` | no direct lock; permission/admission/call site (production; change) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:232` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:250` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:254` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:259` | FOR UPDATE: tickets (production; change_in) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:268` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:285` | FOR UPDATE: tickets (production; change_in) | 4 (domain order in README) | yes | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:298` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:302` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:303` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:355` | no direct lock; permission/admission/call site (production; change_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:410` | FOR UPDATE: tickets (production; show_page) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:459` | FOR UPDATE SKIP LOCKED: tickets (production; clean) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:152` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; prepare) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:158` | FOR UPDATE: tickets (production; prepare) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:160` | FOR UPDATE: player_reports (production; prepare) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:162` | FOR UPDATE: sanctions (production; prepare) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:164` | FOR UPDATE: sanctions (production; prepare) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:167` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; prepare) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:177` | FOR UPDATE: support_appeal_workflows (production; prepare) | 4 (domain order in README) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:186` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; prepare) | 3 → 4 → 5 → 6 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:336` | no direct lock; permission/admission/call site (production; apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:89` | no direct lock; permission/admission/call site (test fixture/barrier; configured_staff_fixture) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:132` | pg_advisory_xact_lock: 71010 (test fixture/barrier; configured_staff_fixture) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:532` | FOR UPDATE: sanctions (test fixture/barrier; configured_staff_fixture) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:648` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_ticket_preparation_and_player_note_share_account_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:671` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_ticket_preparation_and_player_note_share_account_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:698` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_ticket_preparation_and_player_note_share_account_order) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:230` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_replies_status_ownership_and_internal_privacy) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:382` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_notification_failure_rolls_back_and_preferences_apply) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:624` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:630` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:707` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_routes_require_host_mfa_permission_and_csrf) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:884` | no direct lock; permission/admission/call site (test fixture/barrier; staff_waiting_mutation_rechecks_authority) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:885` | no direct lock; permission/admission/call site (test fixture/barrier; staff_waiting_mutation_rechecks_authority) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:909` | FOR UPDATE: accounts (test fixture/barrier; staff_waiting_mutation_rechecks_authority) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:933` | FOR UPDATE: pg_stat_activity, accounts (test fixture/barrier; staff_waiting_mutation_rechecks_authority) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:947` | no direct lock; permission/admission/call site (test fixture/barrier; staff_waiting_mutation_rechecks_authority) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:953` | no direct lock; permission/admission/call site (test fixture/barrier; staff_waiting_mutation_rechecks_authority) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1075` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_pagination_ticket_boundary) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1134` | no direct lock; permission/admission/call site (test fixture/barrier; lk_crossed_staff_replies_finish) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1252` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_support_refuses_session_revoked_during_account_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:49` | no direct lock; permission/admission/call site (production; active_verified_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:86` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:153` | no direct lock; permission/admission/call site (production; staff_routes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/ui/src/lib.rs:244` | no direct lock; permission/admission/call site (production; text) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/ui/src/lib.rs:434` | no direct lock; permission/admission/call site (test fixture/barrier; language_switch_uses_mounted_staff_route_shapes) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:10` | FOR UPDATE (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:66` | FOR UPDATE: coin_purchase_reversals (production; settle) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:100` | FOR UPDATE (production; settle) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:102` | no direct lock; permission/admission/call site (production; settle) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:111` | no direct lock; permission/admission/call site (production; unlocked_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/eligibility.rs:7` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/eligibility.rs:15` | no direct lock; permission/admission/call site (production; value_eligible_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/eligibility.rs:17` | FOR SHARE: accounts (production; safe_return_in) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:59` | no direct lock; permission/admission/call site (production; allowed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:60` | no direct lock; permission/admission/call site (production; allowed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:61` | no direct lock; permission/admission/call site (production; allowed) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:69` | no direct lock; permission/admission/call site (production; balance) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:74` | no direct lock; permission/admission/call site (production; balance) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:201` | no direct lock; permission/admission/call site (production; adjust) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:334` | FOR UPDATE: wallet_reservations (production; finish_reservation) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:455` | FOR UPDATE: wallet_accounts (production; lock) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:538` | no direct lock; permission/admission/call site (production; with_adjustment_kind) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:552` | no direct lock; permission/admission/call site (production; read_balance_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:568` | no direct lock; permission/admission/call site (production; snapshot_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:616` | no direct lock; permission/admission/call site (production; known) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:652` | no direct lock; permission/admission/call site (production; reserve_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:664` | no direct lock; permission/admission/call site (production; settle_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:665` | no direct lock; permission/admission/call site (production; settle_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:701` | FOR UPDATE: wallet_reservations (production; settle_existing_in) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:911` | FOR UPDATE: wallet_reservations (production; settle) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:103` | no direct lock; permission/admission/call site (production; adapt) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:530` | no direct lock; permission/admission/call site (production; safe_return) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:538` | FOR SHARE: accounts (production; state) | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:549` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:550` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:551` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:552` | no direct lock; permission/admission/call site (production; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:75` | no direct lock; permission/admission/call site (test fixture/barrier; call) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:476` | no direct lock; permission/admission/call site (test fixture/barrier; flags_transport_and_peer_provenance_fail_closed_without_database) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:527` | no direct lock; permission/admission/call site (test fixture/barrier; flags_transport_and_peer_provenance_fail_closed_without_database) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:21` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:162` | no direct lock; permission/admission/call site (test fixture/barrier; top_up) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:28` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:30` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:32` | no direct lock; permission/admission/call site (test fixture/barrier; permission) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:650` | no direct lock; permission/admission/call site (test fixture/barrier; one_coin_balance_store_debit_and_audited_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:668` | no direct lock; permission/admission/call site (test fixture/barrier; one_coin_balance_store_debit_and_audited_grants) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:752` | no direct lock; permission/admission/call site (test fixture/barrier; reward_credit_is_spendable_and_transferable_like_a_top_up) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:1045` | no direct lock; permission/admission/call site (test fixture/barrier; l6_04_wallet_safe_release_allows_suspended_holder) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:1050` | no direct lock; permission/admission/call site (test fixture/barrier; l6_04_wallet_safe_release_allows_suspended_holder) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/README.md:14` | for share (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/lib.rs:104` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/lib.rs:198` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/ratelimit.rs:442` | FOR UPDATE SKIP LOCKED (production; clean) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/session.rs:225` | no direct lock; permission/admission/call site (production; set) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/web/src/session.rs:296` | FOR UPDATE: accounts (production; lock_session_accounts) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/web/src/session.rs:323` | no direct lock; permission/admission/call site (production; rotate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/web/src/session.rs:337` | no direct lock; permission/admission/call site (production; rotate) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/web/src/session.rs:497` | FOR UPDATE SKIP LOCKED: web_sessions (production; clean) | 6 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/web/src/session_tests.rs:552` | FOR UPDATE: accounts (test fixture/barrier; postgres_drc_rotation_accounts_precede_session_trigger_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/session_tests.rs:573` | FOR UPDATE NOWAIT: web_sessions (test fixture/barrier; postgres_drc_rotation_accounts_precede_session_trigger_wait) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/README.md:10` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:18` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:24` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:150` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_entity_editorial_publication_and_locales) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:371` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_ambiguous_editorial_binding_denies) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/places/README.md:23` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:5` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:37` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:42` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:46` | no direct lock; permission/admission/call site (production; new) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:54` | no direct lock; permission/admission/call site (production; authorize) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:80` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:86` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_in) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:91` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_in) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:94` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_in) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:95` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; authorize_in) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:100` | no direct lock; permission/admission/call site (production; list) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:108` | no direct lock; permission/admission/call site (production; get) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:122` | no direct lock; permission/admission/call site (production; history) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:133` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:141` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:147` | pg_advisory_xact_lock: 20261105220206 (production; create) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:156` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:157` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:158` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:163` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:181` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; create) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:194` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:202` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:208` | pg_advisory_xact_lock: 20261105220206 (production; edit) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:211` | FOR UPDATE: wiki_guides (production; edit) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:221` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:222` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:223` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:230` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:246` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; edit) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:259` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:262` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:268` | pg_advisory_xact_lock: 20261105220206 (production; restore) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:271` | FOR UPDATE: wiki_guides (production; restore) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:281` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:282` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:283` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:289` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:307` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; restore) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:319` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:322` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:324` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:330` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:331` | FOR UPDATE: wiki_guides (production; transition) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:333` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:336` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:337` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:339` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:341` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:344` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:368` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:372` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:374` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; transition) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:389` | FOR UPDATE: wiki_guides (production; lock) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/repository.rs:400` | pg_advisory_xact_lock: 20261105220206 (production; slug_lock) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/wiki/src/web.rs:32` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:61` | no direct lock; permission/admission/call site (production; staff_router) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:306` | no direct lock; permission/admission/call site (production; create_form) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:563` | no direct lock; permission/admission/call site (production; preview) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:29` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:35` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:82` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:349` | no direct lock; permission/admission/call site (test fixture/barrier; database_refusals_bad_input_and_concurrent_edits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:356` | no direct lock; permission/admission/call site (test fixture/barrier; database_refusals_bad_input_and_concurrent_edits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:367` | no direct lock; permission/admission/call site (test fixture/barrier; database_refusals_bad_input_and_concurrent_edits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:372` | no direct lock; permission/admission/call site (test fixture/barrier; database_refusals_bad_input_and_concurrent_edits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:410` | no direct lock; permission/admission/call site (test fixture/barrier; database_refusals_bad_input_and_concurrent_edits) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:742` | no direct lock; permission/admission/call site (test fixture/barrier; database_queued_create_edit_restore_recheck_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:751` | pg_advisory_xact_lock: 20261105220206 (test fixture/barrier; database_queued_create_edit_restore_recheck_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:774` | pg_advisory_xact_lock: pg_stat_activity, 20261105220206 (test fixture/barrier; database_queued_create_edit_restore_recheck_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:780` | pg_advisory_xact_lock: 71010 (test fixture/barrier; database_queued_create_edit_restore_recheck_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:784` | no direct lock; permission/admission/call site (test fixture/barrier; database_queued_create_edit_restore_recheck_revocation) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:920` | FOR UPDATE: wiki_guides (test fixture/barrier; postgres_a3_wiki_edit_and_restore_wait_before_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:949` | pg_try_advisory_xact_lock: 71010 (test fixture/barrier; postgres_a3_wiki_edit_and_restore_wait_before_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:2` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:12` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:18` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:68` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:101` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:102` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:103` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:104` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:106` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:112` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:133` | FOR UPDATE: wiki_guides (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:150` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:151` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:158` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:159` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:160` | no direct lock; permission/admission/call site (test fixture/barrier; publish_refuses_permission_revoked_while_waiting_on_guide) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md:57` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/README.md:87` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/README.md:137` | no direct lock; permission/admission/call site (documentation/data; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:15` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:27` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:43` | no direct lock; permission/admission/call site (production; configured_router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:63` | no direct lock; permission/admission/call site (production; router) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:72` | no direct lock; permission/admission/call site (production; routes) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:185` | no direct lock; permission/admission/call site (production; page) | n.a. | n.a. | n.a. | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:307` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:308` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:309` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:312` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:313` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:318` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:331` | FOR UPDATE: world_active_version (production; mutate) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:336` | FOR UPDATE: world_catalog_publication (production; mutate) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:348` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:349` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:350` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:360` | FOR UPDATE: world_active_version (production; mutate) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:383` | FOR UPDATE: world_catalog_publication (production; mutate) | 4 (domain order in README) | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:390` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:391` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:392` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:396` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:504` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:505` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:506` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:510` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin.rs:517` | ordered accounts UPDATE → prepared resources → RBAC exclusive → session/MFA UPDATE (owning writer protocol) (production; mutate) | 3 → 4 → 5 → 6 | yes | yes | fixed in this commit |
| `portal-rs/crates/world/src/admin_pool_tests.rs:2` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:12` | no direct lock; permission/admission/call site (test fixture/barrier; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:18` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:68` | no direct lock; permission/admission/call site (test fixture/barrier; open) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:120` | pg_advisory_xact_lock: pg_stat_activity, pg_database, 71010 (test fixture/barrier; wait_for_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:134` | no direct lock; permission/admission/call site (test fixture/barrier; world_two_queued_mutations_complete_with_two_connections) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:138` | pg_advisory_xact_lock: 71010 (test fixture/barrier; world_two_queued_mutations_complete_with_two_connections) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:152` | pg_advisory_xact_lock, FOR UPDATE: pg_stat_activity, pg_database, world_active_version, 71010 (test fixture/barrier; world_two_queued_mutations_complete_with_two_connections) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:176` | no direct lock; permission/admission/call site (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:181` | no direct lock; permission/admission/call site (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:189` | no direct lock; permission/admission/call site (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:190` | no direct lock; permission/admission/call site (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:191` | no direct lock; permission/admission/call site (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:215` | pg_advisory_xact_lock: 71010 (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:241` | no direct lock; permission/admission/call site (test fixture/barrier; world_refuses_revoked_prerequisite_with_action_retained) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:262` | no direct lock; permission/admission/call site (test fixture/barrier; publication_wait_does_not_hold_global_authorization_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:269` | no direct lock; permission/admission/call site (test fixture/barrier; publication_wait_does_not_hold_global_authorization_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:276` | FOR UPDATE (test fixture/barrier; publication_wait_does_not_hold_global_authorization_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:291` | FOR UPDATE: pg_stat_activity, pg_database (test fixture/barrier; publication_wait_does_not_hold_global_authorization_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:297` | no direct lock; permission/admission/call site (test fixture/barrier; publication_wait_does_not_hold_global_authorization_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:315` | no direct lock; permission/admission/call site (test fixture/barrier; publication_wait_does_not_hold_global_authorization_fence) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:449` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_a3_world_fence_deadline_releases_publication_pointers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:453` | pg_advisory_xact_lock: 71010 (test fixture/barrier; postgres_a3_world_fence_deadline_releases_publication_pointers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:467` | FOR UPDATE: world_active_version (test fixture/barrier; postgres_a3_world_fence_deadline_releases_publication_pointers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:473` | FOR UPDATE: world_catalog_publication (test fixture/barrier; postgres_a3_world_fence_deadline_releases_publication_pointers) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:52` | no direct lock; permission/admission/call site (test fixture/barrier; resolve) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:58` | no direct lock; permission/admission/call site (test fixture/barrier; authenticated_in) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:148` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_publication_visibility_names_diff_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:431` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_publication_visibility_names_diff_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:446` | no direct lock; permission/admission/call site (test fixture/barrier; postgres_staff_publication_visibility_names_diff_and_refusals) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/cache.rs:38` | no direct lock; permission/admission/call site (production; record) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/mod.rs:593` | FOR UPDATE: world_catalog_candidates (production; persist_inactive) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/publication.rs:78` | FOR UPDATE: world_catalog_publication (production; reserve_publication) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/publication.rs:136` | FOR UPDATE: world_catalog_publication (production; publish) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/sprites/mod.rs:370` | no direct lock; permission/admission/call site (production; generate) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/store.rs:13` | no direct lock; permission/admission/call site (production; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/store.rs:281` | FOR UPDATE: world_active_version (production; lock_active) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/crates/world/tests/postgres.rs:245` | FOR UPDATE: world_active_version (test fixture/barrier; concurrent_switches_wait_for_the_same_lock_and_keep_the_actual_predecessor) | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/tests/postgres.rs:494` | no direct lock; permission/admission/call site (test fixture/barrier; real_pinned_game_tree_imports_with_its_own_schemas) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:9` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:47` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:54` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:78` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:113` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:121` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:139` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:147` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:155` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105220212_t05_password_resets.sql:18` | FOR UPDATE: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105230107_t64_moderation.sql:82` | pg_advisory_xact_lock_shared: 64002 (migration; module/SQL) | 1 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105230107_t64_moderation.sql:90` | FOR UPDATE: bazaar_listings (migration; module/SQL) | 2 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231209_t89_season_pass.sql:1` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:2` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:3` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:70` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:84` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:101` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:123` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:142` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:165` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:174` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:183` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:195` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:1` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:2` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:3` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:4` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:2` | pg_advisory_xact_lock: 71140002 (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:58` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:59` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:126` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:138` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107170001_admin_analytics_economy.down.sql:4` | pg_advisory_xact_lock: 71140001 (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107170001_admin_analytics_economy.up.sql:2` | pg_advisory_xact_lock: 71140001 (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107180001_wallet_topup.down.sql:2` | LOCK TABLE: wallet_operations (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:130` | FOR UPDATE: support_sanction_export_counter (migration; module/SQL) | 7 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:150` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:168` | FOR UPDATE: support_sanction_export_counter (migration; module/SQL) | 7 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:209` | FOR UPDATE SKIP LOCKED: support_sanction_export_counter (migration; module/SQL) | 7 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:102` | FOR UPDATE NOWAIT: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:104` | FOR UPDATE: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:109` | FOR UPDATE NOWAIT (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:112` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:123` | pg_advisory_xact_lock: 5600 (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:135` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:161` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:177` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:273` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:98` | FOR UPDATE: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:99` | FOR UPDATE: identity_security_revocation_counter (migration; module/SQL) | 7 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:141` | FOR UPDATE: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:154` | FOR UPDATE SKIP LOCKED: identity_security_revocation_counter (migration; module/SQL) | 7 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:184` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:185` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:253` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112090001_t19c_grant_issuance.sql:6` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100004_store_offers_coins_only.sql:5` | LOCK TABLE: store_offers (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:23` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:24` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:93` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:2` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:3` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:74` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115110002_fix_02_identity.sql:19` | FOR UPDATE: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:24` | pg_advisory_xact_lock_shared: 64002 (migration; module/SQL) | 1 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:35` | pg_advisory_xact_lock_shared: 64002 (migration; module/SQL) | 1 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:41` | FOR UPDATE: bazaar_listings (migration; module/SQL) | 2 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:76` | pg_advisory_xact_lock: 22005160000 (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120002_fix_a2-01b-admin_hold_identity.sql:8` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:2` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:3` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:75` | no direct lock; permission/admission/call site (migration; module/SQL) | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120014_fix_a2-13-world-atlas-wiki_authority.sql:71` | FOR UPDATE: world_catalog_publication (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql:11` | pg_advisory_xact_lock: 15120016 (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql:48` | LOCK TABLE: support_sanction_export_revisions (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql:99` | FOR UPDATE SKIP LOCKED: support_sanction_export_counter (migration; module/SQL) | 7 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:9` | FOR SHARE NOWAIT: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:11` | FOR SHARE: accounts (migration; module/SQL) | 3 | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:66` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:91` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:108` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:17` | pg_advisory_xact_lock_shared: 64002 (migration; module/SQL) | 1 | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:26` | pg_advisory_xact_lock_shared: 64002 (migration; module/SQL) | 1 | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:32` | FOR UPDATE: bazaar_listings (migration; module/SQL) | 2 | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:48` | pg_advisory_xact_lock: 22005160000 (migration; module/SQL) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:70` | pg_advisory_xact_lock: 15120016 (migration; module/SQL) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:106` | FOR UPDATE NOWAIT: accounts (migration; module/SQL) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:108` | FOR UPDATE: accounts (migration; module/SQL) | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:113` | FOR UPDATE NOWAIT (migration; module/SQL) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:116` | FOR UPDATE (migration; module/SQL) | 4 (domain order in README) | yes | n.a. | fixed in this commit |
| `portal-rs/migrations/20261116041001_a3_drc_lock_budgets.sql:129` | pg_advisory_xact_lock: 5600 (migration; module/SQL) | 4 (domain order in README) | yes | n.a. | fixed in this commit |

## Final validation

PostgreSQL **18.6** (`18.6-1.pgdg13+2`) was verified on every test instance;
Rust **1.97.0** matches the repository pin. Independent instances kept concurrent
schema migration fixtures within the original PostgreSQL settings. No PostgreSQL
configuration, timeout, acquisition budget or test assertion was increased or
relaxed. Existing ignored browser tests were executed with `--include-ignored`.

The final target results below include unit and integration tests. Counts exclude
nested fixture-process summaries; each enclosing Cargo test target is counted
once. Earlier failed targets were rerun after repair; passing targets of grouped
commands are retained individually. Documentation was subsequently checked for
all 18 touched libraries in one stable build.

| Package | Passed | Failed | Ignored | Evidence under `/tmp/drc-evidence/` |
|---|---:|---:|---:|---|
| `oteryn-admin` | 174 | 0 | 0 | `final-group-d1.log` |
| `oteryn-bazaar` | 133 | 0 | 0 | `final-group-a2.log` |
| `oteryn-characters` | 93 | 0 | 0 | `final-group-b2.log` |
| `oteryn-community` | 161 | 0 | 0 | `final-group-d2.log` |
| `oteryn-content` | 31 | 0 | 0 | `final-group-a2.log` |
| `oteryn-db` | 9 | 0 | 0 | `final-group-c2.log` |
| `oteryn-downloads` | 17 | 0 | 0 | `final-group-a2.log` |
| `oteryn-gameauth` | 156 | 0 | 0 | `final-group-b2.log` |
| `oteryn-identity` | 167 | 0 | 0 | `final-group-b2.log` |
| `oteryn-minigames` | 67 | 0 | 0 | `final-group-e1.log` |
| `oteryn-payments` | 46 | 0 | 0 | `final-group-b2.log` |
| `oteryn-playerdata` | 157 | 0 | 0 | `final-playerdata2.log` |
| `oteryn-portal` | 192 | 0 | 0 | `final-group-e1.log` |
| `oteryn-stats` | 56 | 0 | 0 | `final-group-b2.log` |
| `oteryn-store` | 121 | 0 | 0 | `final-group-d2.log` |
| `oteryn-support` | 94 | 0 | 0 | `final-group-a2.log` |
| `oteryn-web` | 60 | 0 | 0 | `final-group-b2.log` |
| `oteryn-wiki` | 37 | 0 | 0 | `final-group-a2.log` |
| `oteryn-world` | 63 | 0 | 0 | `final-group-d1.log` |
| **Total** | **1834** | **0** | **0** | 18 libraries plus portal bin/integration targets |

- `cargo +1.97.0 fmt --check`: PASS (`fmt-final.log`).
- `cargo +1.97.0 clippy --workspace --all-targets -- -D warnings`: PASS (`clippy-final.log`).
- Grouped `cargo +1.97.0 test -p <packages> --no-fail-fast -- --include-ignored --test-threads=1`: final per-package targets above PASS.
- `cargo +1.97.0 test -p <all 18 libraries> --doc -- --include-ignored --test-threads=1`: PASS, 18 documentation targets (`final-docs.log`).
- All **12** dedicated `postgres_drc_` regressions PASS. Seven originally failed against the requested base; the red logs are `red.log`, `red-oteryn-bazaar.log`, `direct-red-store.log`, `direct-red-portal.log`, `direct-red-oteryn_playerdata-.log`, `direct-red-command-.log`, and `red-budget.log`.

The writer closeout additionally covers code generation, party moderation,
catalog import/activation, stored control previews/approvals and evidence.
Control execution and referenced preview rows are prepared together in UUID
order before actor/RBAC acquisition. The hierarchy remains exclusively in the
RBAC README; the site inventory records each owning protocol.

Budget propagation exposed fixture calls that used transaction-owned account
creation or ownership evidence on bare connections. Those fixtures now retain
and commit/roll back their actual transactions; assertions remain unchanged.
Native signing setup samples its activation clock after loading trust. The final
admission write discards its prepared grant if the bounded issuance window times
out, requiring a fresh login.

The unchanged 257-record promotion test initially exhausted the shared deadline.
Promotion now reuses retained, locked subject cuts within each 128-row batch and
checks database time for every record; ordinary login activity is captured by its
existing trigger, while expired payloads retain the explicit activity high-water
mark. The full 157-test playerdata command was repeated successfully after an
intermediate concurrent-build documentation-cache failure.

The existing fourteen-season, 14,000-guild portal test also reproduced a deadline
failure in isolation (`league-quiet.log`). ASCII guild names now skip Unicode
normalization and category-table lookups; ASCII control rejection and the entire
Unicode path are unchanged. A new validation test covers all ASCII controls,
non-normalized Unicode and format characters. The original full-page assertions
and three-second deadline pass (`league-ascii-green.log`, `final-group-e1.log`).

Applied migrations are unchanged. Only additive migration
`20261116041001_a3_drc_lock_budgets.sql` is added. The inventory matches all
**2,670** grep sites with **zero missing and zero extra matching entries**.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-16T07:05:39Z
head: f74eb95f91d28feefba8d02278ff5db1c4afe61d
branch: claude/happy-davinci-ijz3wy-fix-a3-drc
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/
  - docs/agents/tasks/active/OTERYN-20261116-a3-drc.md
proven:
  - PostgreSQL 18.6 verified on every instance
  - Seven red regressions and twelve passing dedicated PostgreSQL regressions
  - 1834 passing tests with zero failed or ignored final targets
  - fmt and workspace all-target Clippy pass on the final source
  - All eighteen library documentation targets pass
  - Inventory covers 2670 matching sites without omissions
  - Applied migrations unchanged
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: Final per-target results recorded above; intermediate failures repaired and rerun
rejected_hypotheses:
  - Compilation contention alone explained the large league-page failure; isolated reproduction required the equivalent ASCII validation optimization
changed_paths:
  - portal-rs/
  - docs/agents/tasks/active/OTERYN-20261116-a3-drc.md
validation:
  - command: cargo fmt --check
    result: PASS
    evidence: /tmp/drc-evidence/fmt-final.log
  - command: cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/drc-evidence/clippy-final.log
  - command: cargo test for all touched crates and portal bin, including ignored browser tests
    result: PASS
    evidence: Final validation matrix above; 1834 passed, zero failed, zero ignored
blockers:
  - none
next_action: Commit and push the validated candidate to the owner-authorized branch; retain it for review.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner-requested review branch; retain for owner review and integration decision.
source_branch_evidence: Owner-authorized ref refs/heads/claude/happy-davinci-ijz3wy-fix-a3-drc; retain for integration review.
```
