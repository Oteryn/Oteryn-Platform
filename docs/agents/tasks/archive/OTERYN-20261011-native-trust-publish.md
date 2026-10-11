---
task_id: OTERYN-20261011-native-trust-publish
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - app/GameAuth/NativeAdmission/NativeAdmissionKeyring.php
  - app/Console/Commands/VerifyNativeAdmissionSigning.php
optional_reads: []
---

# OTERYN-20261011 native admission trust publication at deploy

## Goal

Owner decision D1006: native login is enabled on Synology preproduction with owner key custody, and the owner does nothing by hand. PR #1544 (B7-DEPLOY) generates the issuer signing key at deploy and runs `game-auth:native-admission:self-check`, which verifies against the trust registry. The only publish command, `game-auth:native-trust:publish-key`, runs on the disposable SQLite store only, so the first deploy with native admission on would be refused.

Add `game-auth:native-admission:publish-trust`, which trusts the public half of the loaded current issuer key in the deployment's MariaDB trust registry. Wire the trust scope and high-water witness into Compose, and call the command from deploy between key ensure and the self-check.

## Acceptance criteria

- [x] The command takes no input and derives the public key from the key the issuer loads, through the keyring's owner and mode checks.
- [x] It refuses outside `testing`/`preproduction`, with native admission off, or with no fresh key purpose. A refusal writes nothing and prints one reason line.
- [x] It writes only through `NativeSigningTrustRegistry::publishTrustedKey`, so every registry rule and the high-water witness apply.
- [x] It is idempotent: an already trusted kid with the same key gives `already_trusted` and writes no registry row. Every run still goes through the witnessed write path, so a lost witness or a database restored below it fails closed (review P1 on PR #1546).
- [x] It prints one JSON line with the trust scope, key id and public key. No seed, private key or signature bytes are printed or logged.
- [x] Compose sets `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE=fresh_admission` and puts the witness directory on the retained `platform_storage` volume. `marketplace-scheduler` gets the same values, because scheduled `identity:finalize-terminations` writes through the witness (review P1 on PR #1546). Deploy creates the directory owner-only on every run, whatever the native admission switch says: once the witness is configured, every identity security revocation needs it.
- [x] Full deploy, fast deploy and the `deploy-impact.sh` platform-reconcile route publish after the Platform runs the new image. Publication comes before scheduler reconciliation, smoke, OAuth ensure and any signing self-check. The reconcile route was added after review P1 4242250745 on PR #1546. After P1 4242266267, the same gate also runs after Platform starts in `rollback.sh` (before health-check and promotion), after each Platform recreation in `marketplace-staging.sh`, and inside `_oteryn_recreate_marketplace_runtime` (before the scheduler restarts). With native admission off, publication is skipped. A review sweep before the next push added the gate to `finalize_previous_candidate_if_healthy` (before health-check and promotion). Publication now runs through the image entrypoint, as the signing self-check does, so the signing key id and file env from the key volume apply. The Compose witness lines sit after `QUEUE_CONNECTION`, so this branch merges cleanly with PR #1544. After P1s 4242299989 and 4242299985, every route goes through `oteryn_publish_native_admission_trust_or_stop`. If publication or witness preparation fails, it stops Platform, internal proxy and Gateway (fail closed). The routes are full, fast, previous-candidate, reconcile, rollback, Marketplace switches and Marketplace runtime recreation. The fast-deploy and reconcile failure-restore traps gate the restored Platform the same way. Only after that passes do they start the preserved proxy and Gateway again and reconcile the scheduler.
- [x] After P1 4242324208, ingress comes up only behind the gate. On every route the order is: stop the internal proxy and Gateway, (re)create Platform, run the gate, then start ingress. Only the internal proxy and Gateway hold the Gateway service credential that `POST /internal/v1/game-auth/native-admissions` requires, so no grant can be issued while trust is unpublished or unwitnessed. A failed gate removes the Platform, internal proxy and Gateway containers instead of stopping them, because `restart: always` would bring a stopped container back after a daemon or NAS restart. Both failure-restore traps go through one shared helper with the same order. That helper removes all three containers if it cannot stop ingress. Witness preparation now also requires an owner-only (700) directory that is not a link and is owned by the Platform user (P2 4242202676).
- [x] A closure audit of every route, failure mode and ordering is recorded below, and each route has an ordering test. A class test fails if any Platform start in the five scripts can be reached while ingress is running before the gate.
- [x] The legacy token ratchet passes with no group rising.
- [ ] Exact-head CI passes.

## Closure audit (trust publication, all routes)

Grants are issued only by `POST /internal/v1/game-auth/native-admissions` behind `RequireGatewayServiceCredential`. Only Gateway holds that credential, and it reaches Platform through the internal proxy. In this audit, "ingress" means the internal proxy and Gateway. Platform's own published web port cannot issue grants. The invariant is that ingress never runs while Platform holds trust that is unpublished or not witnessed. "Stop" is `oteryn_native_admission_ingress_stop` and "gate" is `oteryn_publish_native_admission_trust_or_stop`.

Routes:

| # | Route | Before Platform (re)create | Gate | Ingress start | Ordering test |
|---|---|---|---|---|---|
| 1 | Full deploy (normal release) | stop, before `up -d platform` and the migration hooks | after migrate and passport keys, before OAuth ensure and the self-check | `up` of internal proxy, then Gateway, after the gate | `test_full_deploy_*`, class test |
| 2 | Full deploy, first deploy | stop is a no-op (no ingress exists yet) | same as 1 | same as 1 | ingress helper first-deploy test, class test |
| 3 | Full deploy, same-release redeploy or resumed candidate (no migration quiesce) | explicit stop, because the quiesce does not run here | same as 1 | same as 1 | class test |
| 4 | Fast path | stop | right after the Platform recreate | `docker start` of the preserved proxy and Gateway ids | fast/reconcile parametrized test |
| 5 | Fast path restore trap | shared restore helper: stop, or remove all three containers if stop fails | after the previous Platform recreate | `up -d --no-deps internal-proxy gateway` | trap test, restore helper tests |
| 6 | Previous-candidate promotion | stop | before the proxy and Gateway recreate | `up --force-recreate internal-proxy gateway` | previous-candidate test |
| 7 | Reconcile (`deploy-impact.sh`) | stop | right after the Platform recreate | `docker start` of the preserved ids | fast/reconcile parametrized test |
| 8 | Reconcile restore trap | same as 5 | same as 5 | same as 5 | trap test, restore helper tests |
| 9 | `rollback.sh` | stop | after `up -d platform` | `up -d platform internal-proxy gateway`, appearing only once and only after the gate | rollback test, class test |
| 10 | Marketplace enable (two recreates) | stop before the first recreate; ingress stays stopped through the second | after each recreate | proxy recreate and `start gateway` after the second gate | Marketplace switch test |
| 11 | Marketplace prepare-rollback | stop | after the recreate | proxy recreate and `start gateway` | Marketplace switch test |
| 12 | `_oteryn_recreate_marketplace_runtime` (from the migrate wrapper, rollback, traps and scheduler reconcile) | stop, recording the running ids | after the recreate | only the ids that were running before | Marketplace runtime test |

Failure modes (the same on every route):

| Failure | Result |
|---|---|
| Witness directory unset, a link, not mode 700, or not owned by the Platform user | Preparation fails. The gate removes Platform, internal proxy and Gateway, and the route aborts. |
| Witness missing for a registry with trust history, or the database restored below it | `publishTrustedKey` refuses and the command exits 1 (feature tests). Then the same removal and abort. |
| Any other registry refusal: kid bound to another key, revoked kid, two trusted keys | Command exits 1. Then the same removal and abort. |
| Publication exits 0 but prints no `published`/`already_trusted` result | Gate fails. Then the same removal and abort. |
| Platform container missing after the recreate | Gate gets an empty id and fails. Then the same removal and abort. |
| Ingress stop fails | Forward routes abort before Platform starts. The fast and reconcile traps run the restore helper, which removes all three containers if stop fails again. |
| Native admission off | Publication is skipped and the gate passes. The issuer is off, so no grant is possible. |
| Failure after the gate (smoke, self-check, health) | Trust is already published and witnessed. The fast and reconcile traps restore behind the gate (rows 5 and 8). Full, rollback and Marketplace routes abort with ingress stopped (fail closed); a rerun or rollback recovers. |
| Removal fails after a failed gate | Logged. Ingress was already stopped before Platform started, so nothing ran with unproven trust. A later daemon restart could restart these containers (residual 2). |

Residuals, not closable within the owned paths:

1. `NativeAdmissionGrantIssuer::assertSignable()` reads the trusted key without checking the witness on each grant. Closing that needs a PHP change outside `owned_paths`.
2. `repair-synology-autostart.yml` runs in its own concurrency group (`oteryn-synology-autostart-repair`, not `synology-staging-deployment`) and runs `docker start` on the proxy and Gateway. A Docker daemon or NAS restart has the same effect, because of `restart: always`. Either one, during the seconds-long gate window of a deploy, could start ingress that the deploy had stopped. Removed containers are not affected.
3. With PR #1544 merged, its fast-path `oteryn_run_native_admission_self_check` runs after the ingress start. Trust is already published and witnessed at that point, but the check would be stricter before `oteryn_native_admission_ingress_start`.

## Ownership

```yaml
owned_paths:
  - app/Console/Commands/PublishNativeAdmissionTrust.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionPublishTrustCommandTest.php
  - tests/ci/test_synology_native_admission_trust_publication.py
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20261011-native-trust-publish.md
modules:
  - game-auth
  - deploy-synology
dependencies:
  - PR #1544 (B7-DEPLOY) turns native admission on, ensures the key and adds the self-check; it merges in either order with this PR
blockers:
  - none
cross_repository_tasks:
  - Game trust entry for the published key (public key hand-off), reported to the lead
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T21:00:00Z
head: see PR head
branch: claude/native-trust-publish-20261011
pr: 1546
status: completed
context_routes:
  - auth-identity
  - security
  - deploy
owned_paths:
  - app/Console/Commands/PublishNativeAdmissionTrust.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionPublishTrustCommandTest.php
  - tests/ci/test_synology_native_admission_trust_publication.py
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20261011-native-trust-publish.md
proven:
  - The self-check fails before publication and passes after it (feature test).
  - A second run writes nothing. A rotation keeps the retiring key and trusts the new current key.
  - An already trusted key fails closed when the witness was lost or the database was restored below it; both tests fail against the first head 6237e61d.
  - Thirteen refusal cases exit 1 with no registry change and no secret in output or logs. They cover environment, switch, purpose, key file, key id, file mode, seed, a kid bound to another key, a revoked kid, two trusted keys, and an unset or missing witness.
derived:
  - On main, native admission is off, so deploy skips publication. Once #1544 is on, publication runs before the self-check that #1544 adds. The hunks sit away from #1544's insertions, so either merge order works.
unknown:
  - Game's key purpose descriptor value; Platform uses `fresh_admission`.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Lift the disposable-store guard of game-auth:native-trust:publish-key. That command takes an arbitrary operator key and also guards route records; widening it would weaken the U8 guard.
  - Accept a public key argument. Deriving the key from the loaded private key means only the key the issuer signs with can be trusted.
changed_paths:
  - app/Console/Commands/PublishNativeAdmissionTrust.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionPublishTrustCommandTest.php
  - tests/ci/test_synology_native_admission_trust_publication.py
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20261011-native-trust-publish.md
validation:
  - command: php8.5 vendor/bin/pint --test (changed PHP files)
    result: PASS
    evidence: passed
  - command: php8.5 vendor/bin/phpstan analyse (changed PHP files, level 10)
    result: PASS
    evidence: no errors
  - command: php8.5 vendor/bin/phpunit tests/Feature/GameAuth
    result: PASS
    evidence: 283 tests, 22 skipped, 1 pre-existing risky; NativeAdmission 49 tests
  - command: python3 -m pytest tests/ci -k synology
    result: PASS
    evidence: tests/ci plus deploy/synology/tests 276 passed (per-route ordering, ingress-before-gate class test, gate removal, restore helper); mutation checks confirm the class test fails when the rollback, Marketplace runtime or reconcile order is broken; a three-way merge-tree with PR #1544 head f4625b85 and main eafe7076 is clean and passes 294 tests
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline; baseline not raised
blockers:
  - none
next_action: none in Platform. Game consumes the published trust values; a revoke step for a third key rotation is a later follow-up.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR #1546 from this branch; the repository setting delete_branch_on_merge is true, so a merge removes the ref; confirm it is absent after merge
```
