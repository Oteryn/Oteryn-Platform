---
task_id: OTERYN-20261015-preprod-native-evidence-activation
governing_issue: 1623
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
search_first: []
optional_reads: []
---

# Preproduction native evidence activation

## Goal

Retain the owner's explicit 2026-10-15 one-way activation for client QA (Oteryn/Oteryn-Game#1942). Governing Issue: #1623; PR: #1624.

## Acceptance criteria

- [x] Committed preproduction defaults retain activation.
- [x] Missing configuration still resolves to the existing false fallback.
- [x] Existing deployment contract tests and owner checklist reflect activation.
- [x] Authenticated runtime account-security observation returns observed/allowed.
- [ ] Protected integration retains the decision for future deployments.
- [ ] Game tester bootstrap and client acceptance are independently verified.

## Ownership

```yaml
owned_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/active/OTERYN-20261015-preprod-native-evidence-activation.md
modules:
  - native-evidence-deployment
dependencies:
  - OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT
blockers:
  - protected integration pending
cross_repository_tasks:
  - Oteryn/Oteryn-Game#1942
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T07:13:00Z
head: UNKNOWN
branch: fix/preprod-native-evidence-activation-20261015
pr: 1624
status: waiting
context_routes:
  - deploy/synology
owned_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/active/OTERYN-20261015-preprod-native-evidence-activation.md
proven:
  - owner explicitly authorized one-way preproduction activation
  - runtime native evidence is observed over real TLS 1.3 mTLS
  - all 20 existing Synology mTLS contract tests pass after activation change
derived: []
unknown:
  - Game bootstrap remains unavailable on deployed Game build 1d3226477e70
conflicts: []
first_failure:
  marker: activation assertion
  evidence: updated test failed against false default before the defaults change
rejected_hypotheses:
  - bootstrap intent route itself unavailable; authenticated read returned 200
changed_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
validation:
  - command: python deploy/synology/tests/test_platform_mtls_contract.py
    result: PASS
    evidence: 20 tests; baseline PASS, intended activation assertion RED, complete suite GREEN
blockers:
  - protected integration pending
next_action: qualify and integrate PR 1624 through protected Platform gates; retain activation during subsequent deployment
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: PR 1624
```

## Operational evidence

Platform and marketplace-scheduler configuration caches were generated with GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED=true and restricted to mode 0600. This is running-container configuration; the committed defaults change still needs protected integration before redeployment. Do not clear the activated caches into the containers' old false environment or roll the switch back.

No password, key or other credential value is included here. Existing Game node returns unavailable after source activation; PostgreSQL logs also show the already-merged Game temporal-join defect (#2118) still present in the old deployed node. A separate Game preproduction build/deploy was dispatched through its protected workflow; this record does not authorize or claim its completion.
