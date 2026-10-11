---
task_id: OTERYN-20261011-native-staging-enable-1
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
optional_reads: []
---

# OTERYN-20261011 NATIVE-STAGING-ENABLE-1

## Goal

Governing GitHub Issue: #1474 (native-only migration). The native World, channel and route now exist on the NAS preproduction store (world row id 1, channel key `oteryn-staging-1`, route `rt.1.e96ce705077bfb6fc1c580175115b4c0`). Fill the committed non-secret native defaults with the issued ids and turn native admission on for staging.

## Acceptance criteria

- [ ] `deploy/synology/native-defaults.env` carries the native World id, the runtime-status and scope-assignment identity maps (valid JSON) and `OTERYN_STAGING_NATIVE_ADMISSION_ENABLED=true`.
- [ ] Values satisfy `oteryn_native_admission_projection_ready`.
- [ ] No secret or model identifier added; `docs/agents/LEGACY_TOKEN_BASELINE.json` untouched.
- [ ] `tests/ci/test_synology_native_admission_gate.py` passes.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/README.md
  - tests/ci/test_synology_native_admission_gate.py
  - docs/agents/tasks/active/OTERYN-20261011-native-staging-enable-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-native-staging-enable-1.md
modules:
  - deploy
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
updated_at: 2026-10-11T19:00:00Z
head: UNKNOWN
branch: claude/native-staging-enable-1-20261011
pr: 1554
status: validating
context_routes:
  - deploy/synology
owned_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/README.md
  - tests/ci/test_synology_native_admission_gate.py
proven:
  - Resolver returns the new values and the identity maps parse as JSON.
derived: []
unknown:
  - Exact-head CI result.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/README.md
  - tests/ci/test_synology_native_admission_gate.py
validation:
  - command: python3 -m pytest -q tests/ci/test_synology_native_admission_gate.py
    result: PASS
    evidence: 237 synology CI tests passed locally
blockers:
  - none
next_action: Run local checks, push once, wait for CI.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```

## Notes

A repository variable of the same name still overrides the file. Rolling back means reverting this file.
