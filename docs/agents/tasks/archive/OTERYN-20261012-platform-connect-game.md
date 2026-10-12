---
task_id: OTERYN-20261012-platform-connect-game
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
optional_reads: []
---

# OTERYN-20261012 connect the staging Platform to the preprod Game node

## Goal

Governing GitHub Issue: 1474 (native login programme; assumed, lead to confirm). Commit the Platform side of the two Game-node routes that were still unconfigured on Synology staging, native evidence and the bootstrap-intent read, for the preprod node 1 `[platform]` client certificate, without the one-way native evidence activation.

## Acceptance criteria

- [x] `native-defaults.env` names `CN=oteryn-preprod-node-1-native-evidence` (Game manifest `deploy/synology-game/preprod-topology.toml`, `certificates.node_native_evidence`) for native evidence and bootstrap-intent reads; the Game node presents that one certificate for both (Game `serve.rs`).
- [x] Fresh-account binding `platform_security` / `fresh_admission` matches the Game wire; clock uncertainty `1` (0..5); bootstrap-intent TTL `300` (1..300).
- [x] `OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED` stays `false` (one-way, owner decision; `docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`).
- [x] The shared identity is distinct from the runtime-status, scope-assignment and account-character identities (contract test).
- [x] README route table and owner checklist updated; no secret, no model identifier; the legacy token baseline is not raised.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-connect-game.md
modules:
  - deploy-synology
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none; the Game node already presents CN=oteryn-preprod-node-1-native-evidence on its [platform] endpoint
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T00:00:00Z
head: see PR head
branch: claude/platform-connect-game-20261012
pr: see PR
status: completed
context_routes:
  - deploy
owned_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-connect-game.md
proven:
  - Live staging (read-only) already matches the node manifest on world, channel, route version and revision; runtime status, scope assignment and the account-character watermark are fresh. Native evidence and the bootstrap-intent read answer 503 (bindings empty).
  - The node [platform] client certificate subject is CN=oteryn-preprod-node-1-native-evidence; the bootstrap-intent authority row exists.
  - While activation is false, Platform refuses evidence whatever the bindings say (NativeEvidenceSource); the bootstrap-intent read does not depend on activation.
derived:
  - The certificate subject and binding values survive the Postgres P6 cutover; the WorldId-bound runtime-status and scope-assignment identities do not.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Activate native evidence in this change. Activation is one-way and leaves evidence floors on the Game node across the P6 store reset; it is an owner decision.
  - Issue a separate bootstrap-intent certificate. The Game node builds both producers from one [platform] client.
changed_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-connect-game.md
validation:
  - command: python3 -m pytest tests/ci deploy/synology/tests
    result: PASS
    evidence: 362 passed
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: baseline not raised
blockers:
  - none
next_action: none after merge; owner runs Deploy Synology Staging, then decides native evidence activation.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR from this branch; delete_branch_on_merge is true; confirm the ref is absent after merge
```
