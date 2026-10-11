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
- [x] Full deploy, fast deploy and the `deploy-impact.sh` platform-reconcile route publish after the Platform runs the new image. Publication comes before scheduler reconciliation, smoke, OAuth ensure and any signing self-check. The reconcile route was added after review P1 4242250745 on PR #1546. After P1 4242266267, the same gate also runs after Platform starts in `rollback.sh` (before health-check and promotion), after each Platform recreation in `marketplace-staging.sh`, and inside `_oteryn_recreate_marketplace_runtime` (before the scheduler restarts). With native admission off, publication is skipped. A review sweep before the next push added the gate to `finalize_previous_candidate_if_healthy` (before health-check and promotion). Publication now runs through the image entrypoint, as the signing self-check does, so the signing key id and file env from the key volume apply. The Compose witness lines sit after `QUEUE_CONNECTION`, so this branch merges cleanly with PR #1544.
- [x] The legacy token ratchet passes with no group rising.
- [ ] Exact-head CI passes.

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
updated_at: 2026-10-11T18:00:00Z
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
    evidence: tests/ci plus deploy/synology/tests 250 passed; git merge-tree with the PR #1544 head is clean
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
