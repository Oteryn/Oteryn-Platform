# Oteryn Platform public module stale-task lifecycle audit

## Verdict

`AUDIT_COMPLETE_WITH_FINDINGS`

The fifth bounded continuous-audit package found two independent high-confidence, high-risk coordination defects: completed Announcements/Events and Download Center tasks remain in the active task directory, claim product paths and direct agents toward already-merged pull requests.

No historical task, retained branch, product module, workflow, production system or external repository was changed.

## Scope inspected

- active task checkpoints, owned paths and next actions;
- terminal PR state and merge identity;
- expected archive records;
- retained source branches;
- duplicate and current ownership searches;
- separation from systemic Agent Governance Issue #558.

## Finding OPA-GOV-0004 — HIGH

**Title:** Merged Announcements and Events task remains active.  
**Evidence state:** `PROVEN`  
**Confidence:** high  
**Concrete remediation owner:** Issue #561

`OTERYN-20260724-announcements-events` remains `ready`, claims Announcements, Events, migrations, routes, views and tests, and instructs a worker to mark PR #157 ready. PR #157 is already merged as `82a415c5de5727d15186cf0d0d79744fb498e187`; no archive exists and the task branch remains.

Required correction is historical lifecycle only: archive the task with accurate terminal evidence, release every owned path/lease, remove the obsolete action and delete or explicitly classify the retained branch. Product files are forbidden.

## Finding OPA-GOV-0005 — HIGH

**Title:** Merged Download Center task remains active.  
**Evidence state:** `PROVEN`  
**Confidence:** high  
**Concrete remediation owner:** Issue #562

`OTERYN-20260724-download-center` remains `ready`, claims Download Center application, configuration, migration, route, view and test paths, and instructs a worker to review and merge PR #161. PR #161 is already merged as `79858de3949e8d5969207357e6fb92bfaada481f`; no archive exists and the task branch remains.

Required correction is historical lifecycle only: archive the task with accurate terminal evidence, release every owned path/lease, remove the obsolete action and delete or explicitly classify the retained branch. Product files are forbidden.

## Parallelization decision

The two remediation Issues are `parallel_safe` because they have:

- distinct active/archive task files;
- distinct source branches;
- distinct coordination keys;
- no shared paths;
- no product, migration, contract or workflow edits;
- independent PR and terminal evidence.

Each worker must still acquire its deterministic `repair/issue-<number>` branch under claim protocol version 2.

## Relationship to systemic governance

Issue #558 remains the systemic prevention/detection owner. It must teach Agent Governance and Control Room to reject or surface this class of contradiction. It does not own mutation of historical task records. Issues #561 and #562 are therefore not duplicates.

## Audit result

```yaml
audited_head: 86cd5cccb47ebfbe1a77e65c2ba8b6d912acfcc5
domain: public-module-stale-task-lifecycle
findings:
  critical: 0
  high: 2
  medium: 0
  low: 0
  informational: 0
new_issues:
  - 561
  - 562
product_repairs: 0
task_lifecycle_repairs: 0
e2e: NOT_APPLICABLE_WITH_REASON
e2e_reason: documentation-only audit with no product or historical-task mutation
production_operations: none
external_writes: none
```

## Bounded conclusion

This package creates complete concrete ownership for the two already-proven public-module task contradictions. It does not claim that the remaining active-task directory is clean. Further inventory should proceed as separate bounded reconciliation against live PR, branch and archive state.
