---
task_id: OTERYN-20261011-b7-deploy-native-login
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - deploy/synology/compose.yml
  - app/GameAuth/NativeLogin/NativeGameLoginTickets.php
optional_reads: []
---

# OTERYN-20261011 B7-DEPLOY native login on Synology

## Goal

Governing GitHub Issue: #1474 (native-only migration), packet B7-DEPLOY of the removal plan under `docs/agents/programs/`. Make native game login deployable on the Synology preproduction stack without hand-held secrets: the stack runs `APP_ENV=preproduction`, deploy creates and proves the issuer signing key, and the native admission flag is on only because deploy proves the issuer can sign.

## Acceptance criteria

- [x] Synology deployment config, preflight, compose comments and docs run as `APP_ENV=preproduction`; `NativeGameLoginTickets::issue()` and every production guard are unchanged (production still refuses native admission; `GAME_AUTH_LEGACY_LOGIN_ENABLED` is untouched).
- [x] `deploy.sh` runs `game-auth:native-admission:self-check` after migrations and before the Gateway is recreated (and after the Platform-only fast path), failing closed; `health-check.sh` re-runs it whenever the flag is `true`.
- [x] `generate-native-admission-key.sh` idempotently creates the Ed25519 key in the root-only named volume `oteryn-native-admission-keys` (0700 directory, 0600 files owned by the Platform user), never prints the private key, prints only key ids and public keys, and supports `rotate`, `drop-retiring` and `show`.
- [x] `native_admission_enabled` is `"true"` for the Synology stack; deploy refuses unless the key volume, key id and self-check pass. `tests/ci/test_synology_native_admission_gate.py` pins this.
- [x] `docs/agents/LEGACY_TOKEN_BASELINE.json` is untouched and the ratchet passes.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - deploy/synology/**
  - tests/ci/test_synology_native_admission_gate.py
  - docs/agents/tasks/archive/OTERYN-20261011-b7-deploy-native-login.md
modules:
  - game-auth
  - deploy
dependencies:
  - PR #1542 (self-check command, merged)
blockers:
  - The signing key's public half must exist in the Platform trust registry (and Game) before the self-check can pass; no production-capable trust publication command exists yet (game-auth:native-trust:publish-key is limited to the disposable preproduction store).
cross_repository_tasks:
  - Game trust config consumes the printed public key and key id.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T15:00:00Z
head: UNKNOWN
branch: claude/b7-deploy-20261011
pr: 0
status: completed
context_routes:
  - deploy
  - security
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - deploy/synology/**
  - tests/ci/test_synology_native_admission_gate.py
  - docs/agents/tasks/archive/OTERYN-20261011-b7-deploy-native-login.md
proven:
  - The keyring requires a regular non-symlink file owned by the process user with no group or other bits holding a canonical base64url 32-byte seed; the key tool writes exactly that.
  - The self-check needs a trusted registry row for the key id; the only publish command refuses outside the disposable store.
derived:
  - A Docker named volume is the only custody location both the Synology host and the runner can address, because the runner sees only the state directory by bind mount.
unknown:
  - Whether the live trust registry already holds the key id the first deploy generates.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Key ids as .env or CI variables: other scripts recreate Platform with their own rendered env files and would drop them; the entrypoint reads ids from the volume instead.
changed_paths:
  - deploy/synology/**
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - tests/ci/test_synology_native_admission_gate.py
validation:
  - command: python3 -m pytest tests/ci -k synology or deploy or workflow
    result: PASS
    evidence: 148 passed
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: bash -n on changed scripts; key tool exercised locally through ensure, rotate and drop-retiring
    result: PASS
    evidence: 0600 files owned by www-data, idempotent, no seed in output
blockers:
  - trust registry publication path for the generated public key (see Ownership)
next_action: none after merge; publish the printed public key to the trust registry and Game.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR from this branch; the repository setting delete_branch_on_merge is true, so a merge removes the ref; confirm it is absent after merge
```
