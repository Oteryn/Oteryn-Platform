# Main integrity policy audit evidence

## Identity

- Programme: `OTERYN_PLATFORM_CONTINUOUS_AUDIT`
- Task: `OTERYN-20260805-main-integrity-policy-audit`
- Repository: `blakinio/Oteryn-Platform`
- Audited default branch: `main`
- Audited head: `a7eb03d49e328e8115adb54e772c9c8366b737d3`
- Finding: `OPA-GOV-0001`
- Finding Issue: #552

## Authoritative live evidence

| Source | Proven fact |
|---|---|
| `GET /repos/blakinio/Oteryn-Platform/branches/main` | `protected=false`; `protection.enabled=false`; required status-check enforcement is `off`; required contexts/checks are empty. |
| `GET /repos/blakinio/Oteryn-Platform/rulesets` | Repository ruleset inventory is `[]`. |
| Repository metadata | Merge commit, rebase and squash are enabled; update-branch and auto-merge are disabled. These merge preferences do not protect `main`. |
| `docs/agents/PROMPTING_STANDARD.md` | Material delivery requires exact-head CI, audit, E2E and terminal PR/task lifecycle. |
| `docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md` | Completion is forbidden while final required CI, review and PR hygiene are incomplete. |

The dedicated branch-protection detail endpoint returned `403 Resource not accessible by integration`. The finding does not depend on that endpoint because the branch response explicitly reports that protection is disabled and the complete repository ruleset inventory is empty.

## Duplicate and ownership search

- Searched open and closed Issues for branch protection, protected main, repository rulesets, direct-push prevention and required status checks: no duplicate root-cause Issue.
- Searched repository files for a canonical ruleset or branch-protection policy: none found.
- Open PR #542 owns native-protocol paths, PR #541 owns the public-domain checkpoint, and PR #550 owns architecture-authority documentation; none owns repository settings for `main`.
- Dependabot and historical task PRs do not constitute an active owner for this gap.

## Evidence classification

```yaml
finding_id: OPA-GOV-0001
severity: high
confidence: high
evidence_state: PROVEN
current_enforcement: none
rulesets: 0
branch_protected: false
required_checks_enforced: false
repository_setting_mutation_by_audit: none
production_mutation: none
external_repository_write: none
```

## Validation boundary

This package records live repository-administration evidence and changes documentation only. It does not test an unauthorized direct push and does not modify repository settings. A negative direct-update test belongs to the separately authorized remediation/verification task after a ruleset policy is approved.
