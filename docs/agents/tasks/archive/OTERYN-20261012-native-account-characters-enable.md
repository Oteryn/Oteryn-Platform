---
task_id: OTERYN-20261012-native-account-characters-enable
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
optional_reads: []
---

# OTERYN-20261012 enable native account characters on Synology

## Goal

Governing GitHub Issue: 1474 (native login programme; assumed, lead to confirm). Enable account-character projection in the committed Synology staging defaults and drop the D171 World tie so the client directory lists the World from the native route record.

## Acceptance criteria

- [x] `compose.yml` pins `GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED=true`, source authority `oteryn:character-authority:primary` and identities `["CN=oteryn-preprod-character-projection"]`, and the D171 ownership fallback off.
- [x] `native-defaults.env` no longer sets `OTERYN_STAGING_NATIVE_WORLD_ID`; deploy no longer refuses native admission for a missing World UUID.
- [x] README and contract tests updated; no secret, no model identifier; the legacy token baseline is not raised.
- [x] Health check fails closed unless the account-character feed is live (review P1).
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - tests/ci/test_synology_native_admission_gate.py
  - docs/agents/tasks/archive/OTERYN-20261012-native-account-characters-enable.md
modules:
  - deploy-synology
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Game node must present the CN=oteryn-preprod-character-projection certificate for the projection push
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T00:00:00Z
head: see PR head
branch: claude/native-account-characters-enable-20261012
pr: see PR
status: completed
context_routes:
  - deploy
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - tests/ci/test_synology_native_admission_gate.py
  - docs/agents/tasks/archive/OTERYN-20261012-native-account-characters-enable.md
proven:
  - Compose pins the three account-character settings and the D171 flag off; the projection-ready check passes without a World UUID and still refuses missing runtime-status or scope-assignment identities (contract tests).
derived:
  - The optional World variable stays an input of the opt-in route publication step only; the workflows keep rendering it.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Remove the World variable from workflows and route publication. Route publication still compares the issued WorldId with it and is a separate opt-in.
changed_paths:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - tests/ci/test_synology_native_admission_gate.py
  - docs/agents/tasks/archive/OTERYN-20261012-native-account-characters-enable.md
validation:
  - command: python3 -m pytest tests/ci deploy/synology/tests
    result: PASS
    evidence: 362 passed
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: baseline not raised
blockers:
  - none
next_action: none after merge; redeploy staging.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR from this branch; delete_branch_on_merge is true; confirm the ref is absent after merge
```
