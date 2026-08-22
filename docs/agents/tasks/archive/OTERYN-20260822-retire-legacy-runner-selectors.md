---
task_id: OTERYN-20260822-retire-legacy-runner-selectors
status: completed
phase: closeout
issue: 1220
pull_request: 1221
branch: none
merged_sha: 1da7ba2d5cf698cd205c1c5ada2fa31da39520cd
---

# Legacy Platform runner selector retirement — terminal closeout

## Result

PR #1221 migrated all eight retained Platform jobs from `runs-on: oteryn-staging` to `platform-runners` + `oteryn-platform` and merged as `1da7ba2d5cf698cd205c1c5ada2fa31da39520cd`.

After final Platform trusted-main PASS, GitHub repo runner `oteryn-synology-staging` (`id=21`) was deleted, the Synology legacy container was removed, and its Compose file was retired. The legacy config/work volumes and Platform state directory were preserved.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T10:25:07Z
head: 1da7ba2d5cf698cd205c1c5ada2fa31da39520cd
branch: none
pr: 1221
status: completed
context_routes:
  - ci-runner-routing
  - deployment-operations
owned_paths: []
proven:
  - PR 1221 merged as 1da7ba2d5cf698cd205c1c5ada2fa31da39520cd.
  - No retained Platform workflow uses the legacy runner selector.
  - Platform diagnostics run 32567509732 job 97018190282 succeeded on the replacement route.
  - Repository runner API returned total_count 0 after deleting legacy runner id 21.
  - Legacy Synology container is absent while both named volumes and Platform state remain preserved.
  - Branch hygiene closeout reports 10 ordinary branches, zero unexplained branches and zero findings; three unique historical heads are retained under refs/oteryn-recovery/20260822/*.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: retained-legacy-selectors
  evidence: eight Platform workflows targeted oteryn-staging before PR 1221.
rejected_hypotheses:
  - Compose project identifier oteryn-staging needed renaming; it remains application state identity, not runner routing.
changed_paths:
  - docs/agents/reports/OTERYN-20260822-runner-closeout-managed-recovery.json
validation:
  - command: exact legacy runner selector scan on merged main
    result: PASS
    evidence: no retained runs-on oteryn-staging selector remains
  - command: final trusted-main Platform diagnostics
    result: PASS
    evidence: run 32567509732 job 97018190282
  - command: post-retirement Synology and GitHub runner audit
    result: PASS
    evidence: three organization runners Up; legacy container absent; repo runner count zero; rollback volumes/state preserved
  - command: branch hygiene and managed recovery verification
    result: PASS
    evidence: zero unexplained ordinary branches; three exact recovery refs match recorded source SHAs
blockers: []
next_action: No action; task archived and parent runner split may close.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: terminal selector migration and legacy retirement complete
source_branch_evidence: PR 1221 merge 1da7ba2d5cf698cd205c1c5ada2fa31da39520cd plus post-retirement audit
```
