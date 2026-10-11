---
task_id: OTERYN-20261011-native-admission-deploy-followup
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/generate-native-admission-key.sh
optional_reads: []
---

# OTERYN-20261011 native admission deploy follow-up

## Goal

Governing GitHub Issue: #1474 (native-only migration). Close the follow-ups left by #1544 and #1546 in one change: staged-key rotation as a separate path, error propagation in the previous-Platform restore, the fast-path self-check before ingress starts, the autostart repair in the deployment concurrency group, and the archive state of the two merged task records.

## Acceptance criteria

- [x] `generate-native-admission-key.sh stage` creates a never-used `next` key that Platform never loads (no restart, idempotent) and prints its key id and public key; `promote` stops ingress, makes next current and current retiring, recreates Platform, publishes and proves trust, and only then starts ingress. The `rotate` guard is unchanged.
- [x] `oteryn_restore_previous_platform_behind_trust_gate` returns failure when the internal proxy and Gateway cannot start or the Marketplace runtime cannot be reconciled.
- [x] The fast path runs the signing self-check before `oteryn_native_admission_ingress_start`.
- [x] `repair-synology-autostart.yml` uses the concurrency group `synology-staging-deployment`.
- [x] The task records of #1544 and #1546 are archived (#1546 already was; #1544 is moved here with its closeout).
- [x] `docs/agents/LEGACY_TOKEN_BASELINE.json` is untouched.
- [ ] Exact-head CI passes.

## Closure audit

| Route or script | Class checked | Result |
| --- | --- | --- |
| Full deploy | self-check before ingress | unchanged, already before the proxy and Gateway start |
| Fast path | self-check before ingress start | moved before `oteryn_native_admission_ingress_start`; a failure triggers the restore trap with ingress still stopped |
| Reconcile | self-check before ingress | already inside the health-check after the gate; unchanged |
| Restore helper (fast and reconcile traps) | every step's failure reaches the caller | proxy/Gateway start and scheduler reconcile failures now return 1 |
| `promote` | ingress stopped across the role swap, recreate, publish, self-check | any failure leaves ingress stopped or removed (fail closed); a swap failure restarts the preserved ingress because nothing changed |
| `stage` | Platform unaffected | no restart, no env change, key never exported by the entrypoint |
| `rotate` | live guard | unchanged, still needs `--maintenance` while admission is live |
| Autostart repair workflow | races with the gate window | same group as the deploy, so it cannot start ingress mid-deploy |
| Marketplace runtime recreation (all routes: full deploy, fast path, reconcile, rollback, promote) | the final Platform is self-checked before ingress returns | `_oteryn_recreate_marketplace_runtime` self-checks the replacement (when its env has admission on) before `ingress_start`; promote reconciles Marketplace before its own self-check |
| `drop-retiring` | retired key stays trusted in the registry | revokes the retiring kid in the trust registry (permanent) and verifies it is untrusted before the key files are destroyed; refuses and keeps the key if no Platform runs or revocation fails, so the next staged promotion has a free trust slot |
| `promote` propagation wait | garbage or short value reopens ingress at once | `OTERYN_NATIVE_TRUST_PROPAGATION_SECONDS` must be an integer of at least 10; validated before any mutation (exit 2) |

Follow-up, out of scope here (owned by PHP paths): a witness check in `NativeAdmissionGrantIssuer::assertSignable`.

## Ownership

```yaml
owned_paths:
  - deploy/synology/**
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_native_admission_gate.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - docs/agents/tasks/active/OTERYN-20261011-native-admission-deploy-followup.md
  - docs/agents/tasks/archive/OTERYN-20261011-b7-deploy-native-login.md
modules:
  - deploy
dependencies:
  - PR #1544 and PR #1546 (merged)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T18:00:00Z
head: UNKNOWN
branch: claude/native-admission-deploy-followup-20261011
pr: 1548
status: validating
context_routes:
  - deploy
  - security
owned_paths:
  - deploy/synology/**
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_native_admission_gate.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - docs/agents/tasks/active/OTERYN-20261011-native-admission-deploy-followup.md
  - docs/agents/tasks/archive/OTERYN-20261011-b7-deploy-native-login.md
proven:
  - The Synology and workflow CI tests pass with the new ordering and guards.
derived:
  - Staged rotation needs no Platform code: the entrypoint exports only current and retiring, and publish-trust publishes the current key.
unknown:
  - Behaviour on a real Synology Docker daemon.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Publish the staged key before promotion through a second Platform container. That needs a database-connected throwaway container and a second code path; promotion behind a stopped ingress gives the same guarantee.
changed_paths:
  - deploy/synology/scripts/generate-native-admission-key.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - .github/workflows/repair-synology-autostart.yml
validation:
  - command: python3 -m pytest tests/ci -k "synology or deploy or workflow or autostart"
    result: PASS
    evidence: see the PR
blockers:
  - none
next_action: wait for CI and review; then fill closeout and archive.
```
