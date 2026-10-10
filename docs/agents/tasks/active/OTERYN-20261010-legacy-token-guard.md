---
task_id: OTERYN-20261010-legacy-token-guard
governing_issue: 1474
required_reads: []
search_first:
  - tools/validation/workflow_inventory.py
optional_reads: []
---

# OTERYN-20261010 legacy token ratchet guard

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet A1 of the removal programme.

Add a CI ratchet that counts the legacy-engine token per path group (file contents, matching lines) plus one group for tracked path names, compares against a committed baseline and fails when any enforced group count rises. Falling counts pass. The regex is written with a bracketed letter so the guard never counts itself, and this record avoids the token for the same reason.

## Acceptance criteria

- [x] `tools/validation/legacy_token_ratchet.py` measures per-group counts and the path-name group, and fails on a rise.
- [x] `docs/agents/LEGACY_TOKEN_BASELINE.json` equals the tool output and `git grep -ic` per group on the base head.
- [x] `tools/validation/test_legacy_token_ratchet.py` covers rise, fall, path names, report-only groups and self-match.
- [x] One CI step runs the tests and the guard.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20261010-legacy-token-guard.md
modules:
  - validation tooling
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
updated_at: 2026-10-10T22:00:00Z
head: UNKNOWN
branch: claude/decanary-guard-1-20261010
pr: none
status: validating
context_routes:
  - validation-tooling
owned_paths:
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20261010-legacy-token-guard.md
proven:
  - Baseline totals equal git grep -I -i -c per group on the base head.
derived: []
unknown:
  - Exact-head CI result.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20261010-legacy-token-guard.md
validation:
  - command: python tools/validation/test_legacy_token_ratchet.py
    result: PASS
    evidence: 10 tests OK
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: counts equal baseline
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: CI tooling only, no runtime behavior changes
blockers:
  - none
next_action: Read exact-head CI and hand the review packet to the lead.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Report-only mode exists per group through `enforce: false` in the baseline; every group is enforced at creation. Lower the baseline with `--write` in packets that remove occurrences.
