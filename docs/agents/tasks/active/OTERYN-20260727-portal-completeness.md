---
task_id: OTERYN-20260727-portal-completeness
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
search_first:
  - Issue 240 portal completeness scope
  - active tasks and open PR ownership
  - current public, Identity, Account, Character, Admin and Wiki routes
  - exact Synology staging release identity evidence
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/testing/E2E_COVERAGE_ROADMAP.md
---

# OTERYN-20260727-portal-completeness

## Goal

Coordinate a fail-closed route-by-route completeness audit and bounded remediation programme for every approved Oteryn launch surface, including direct reconciliation of the code deployed at Synology staging with the exact release identity claimed by repository evidence.

## Acceptance criteria

- [ ] A durable completeness matrix inventories every public, Identity, Account, Character, Admin, CMS, Events, Downloads, Wiki, Media and error-state surface.
- [ ] Every matrix row records ownership, user purpose, data dependencies, desktop/tablet/mobile status, localization, accessibility, security, automated coverage and exact-SHA live staging evidence.
- [ ] Every launch-scope surface is classified as `COMPLETE`, `PARTIAL`, `SKELETON`, `BROKEN`, `DEPLOYMENT_DRIFT` or `DEFERRED_WITH_PRODUCT_DECISION`.
- [ ] Direct operator evidence showing Synology output that differs from repository views is reconciled before further staging-completeness claims.
- [ ] Account Center and Character Profile receive the first bounded remediation child tasks.
- [ ] The obsolete public design-preview route is removed or restricted through a bounded presentation-settings child.
- [ ] A homepage-template selector is introduced only through an explicit administrator-owned setting with no public arbitrary template parameter.
- [ ] No `SKELETON`, `BROKEN` or `DEPLOYMENT_DRIFT` row remains unresolved for the approved launch scope.
- [ ] Final exact-head CI and complete exact-SHA Synology browser acceptance pass before Issue #240 closes.

## Ownership

```yaml
owned_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
modules:
  - AgentGovernance
  - Testing
  - PublicPortal
  - Identity
  - Accounts
  - Characters
  - PublicGameData
  - CMS
  - Admin
  - Wiki
  - Deployment
dependencies:
  - Issue 240
  - Issue 91 production boundary
blockers:
  - exact running Synology Platform image SHA is not directly proven from the operator-observed environment
cross_repository_tasks:
  - Canary schema/source remains read-only and may be inspected for field semantics
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T14:20:00Z
head: f5aeb2e80d4692b3ee6309cc3454aa20697721f2
branch: audit/OTERYN-20260727-portal-completeness
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - auth-identity
  - accounts-characters
  - public-game-data
  - admin-rbac
  - security
  - accessibility
  - deployment
owned_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
proven:
  - Issue 240 records the fail-closed full-portal audit and remediation programme
  - current main Account Overview is a narrow provisioning and security summary rather than a complete account-management center
  - current main Character Profile selects and renders only id, name, level and numeric vocation
  - operator-observed Synology Character Profile does not match current main or recorded final public-staging SHA 415aa3febd04c8d9c61082d4a7451352bf084013
  - final public Synology smoke recorded only six public assertions and did not prove every portal surface
  - public design preview route design/home-v2 remains registered on main
derived:
  - deployed release identity must be classified DEPLOYMENT_DRIFT until the running image SHA and file content are reconciled
  - Account Center and Character Profile are SKELETON/PARTIAL launch surfaces and require bounded remediation
unknown:
  - exact running Synology Platform image digest and application SHA for the operator-observed screens
  - complete route-to-state inventory after all module route files are enumerated
  - which additional character fields are safe, authoritative and useful under the current Canary read contract
conflicts:
  - repository/staging completion declarations conflict with direct operator evidence of unfinished screens and code/deployment mismatch
first_failure:
  marker: deployed-character-view-mismatch
  evidence: operator screenshot renders Sex, Vocation, Guild, Residence and Last login while repository character.blade.php at main and SHA 415aa3 renders only Level and numeric Vocation ID
rejected_hypotheses:
  - the observed Character Profile is the exact view from current main: repository source differs materially
  - the final six-route public smoke proves complete portal acceptance: it did not exercise Identity, Account, Character, Admin or most public modules
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
validation:
  - command: repository and staging evidence reconciliation through GitHub connector plus direct operator screenshots
    result: PARTIAL
    evidence: P0 deployment drift and skeletal Account/Character surfaces proven; complete route inventory pending
blockers:
  - exact running Synology Platform image SHA cannot be read through the current connector-only environment
next_action: Create the durable portal completeness matrix and register this coordination task in ACTIVE_WORK.
```

## Notes

This task owns programme coordination and the shared matrix only. Runtime remediation must use separate child tasks and non-overlapping owned paths. Production verification remains Issue #91.