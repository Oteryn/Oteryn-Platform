# Oteryn Platform main integrity policy audit

## Verdict

`AUDIT_COMPLETE_WITH_FINDING`

The second bounded continuous-audit package found one high-confidence, high-risk repository-governance defect: the default branch is not protected and no repository ruleset exists, so the documented PR, exact-head CI, audit, E2E and closeout lifecycle is not technically enforced by GitHub.

No repository setting, workflow, product code, production system or external repository was changed.

## Selection and ownership reconciliation

Live open work was refreshed before selection.

Excluded because actively owned:

- native-protocol implementation and contracts — PR #542;
- public-domain checkpoint — PR #541;
- architecture authority documentation — PR #550.

The repository-integrity setting surface was selected because it is independent of those paths and affects every future Platform delivery. No existing Issue or task owned the root cause.

## Scope inspected

- default branch metadata and protection classification;
- repository ruleset inventory;
- repository merge-method metadata;
- documented exact-head CI, audit, E2E, PR and task closeout requirements;
- open and closed Issues and active PR ownership;
- duplicate and repository-policy searches.

## Finding OPA-GOV-0001 — HIGH

**Title:** Main branch has no protection or ruleset enforcement.  
**Evidence state:** `PROVEN`  
**Confidence:** high  
**Remediation/decision owner:** Issue #552

### Expected

`refs/heads/main` should be covered by an active branch rule that rejects ordinary direct updates, force pushes and deletion, requires the approved pull-request path, and enforces a stable explicit set of exact-head checks before merge. Any emergency bypass should be narrow, intentional and auditable.

### Actual

The branch API reports:

- `protected=false`;
- `protection.enabled=false`;
- required status-check enforcement `off`;
- no required contexts or checks.

The repository ruleset endpoint returns an empty array. Enabled merge methods are preferences only and do not prevent direct updates or enforce validation.

### Impact

A push-capable user, automation token or compromised integration can bypass the repository's documented PR, review, exact-head CI, independent audit, E2E and task-closeout requirements and update `main` directly. Workflow success and human discipline are not an enforcement boundary.

The audit did not attempt an unauthorized direct push. It records the missing control without exploiting it.

### Required acceptance

1. Approve one explicit main-integrity policy and emergency-bypass model.
2. Cover `refs/heads/main` with one active repository ruleset or equivalent branch protection.
3. Require pull requests for ordinary updates.
4. Deny force pushes and branch deletion.
5. Configure stable exact required status checks with no false-green skip or renamed-check gap.
6. Reconcile merge methods, update-branch and auto-merge settings with the accepted policy.
7. Prove positive merge behavior and a negative denied direct-update attempt after apply.
8. Record rollback and recovery procedures before enforcement is considered terminal.

## Authorization boundary

The finding is `implementation_authorized: false` and `state:blocked`. Creating or changing repository rules is an administrative mutation requiring an approved policy and a tool/credential with repository-rules write authority. The continuous auditor is not authorized to perform that mutation under this package.

## Audit result

```yaml
audited_head: a7eb03d49e328e8115adb54e772c9c8366b737d3
domain: main-integrity-policy
findings:
  critical: 0
  high: 1
  medium: 0
  low: 0
  informational: 0
new_issue: 552
repository_setting_repairs: 0
runtime_repairs: 0
e2e: NOT_APPLICABLE_WITH_REASON
e2e_reason: documentation-only audit with no runtime or repository-setting mutation
production_operations: none
external_writes: none
```

## Next audit domain

After terminal closeout of this package, refresh ownership again. Do not audit native protocol while PR #542 is active, public edge while PR #541 is active, or architecture authority while PR #550 is active. The next independent candidates are stale PR/dependency lifecycle integrity and non-overlapping validation/governance infrastructure.
