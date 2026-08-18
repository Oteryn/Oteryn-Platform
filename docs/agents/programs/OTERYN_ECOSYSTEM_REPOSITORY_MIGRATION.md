---
programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
programme_version: 1
canonical_prompt: docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
required_reads:
  - AGENTS.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
repository: blakinio/Oteryn-Platform
owner_alias: OTERYN-REPO-MIGRATION
---

# Oteryn Ecosystem Repository Migration — Programme State

## Mission

Move the accepted Oteryn ecosystem topology from logical ownership to verified, bounded physical repository migration without sacrificing history, provenance, CI/release integrity, rollback, security or product delivery.

## Durable state

```yaml
programme_state_version: 1
updated_at: 2026-08-18T07:24:00Z
status: ready
active_task: OTERYN-20260818-repository-migration-org-access
issue: null
branch: docs/oteryn-20260818-repository-migration-org-access
pull_request: 1143
temporary_topology_authority:
  repository: blakinio/Oteryn-Platform
  path: docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  status: VERIFIED_LIVE_FOR_WAVE_1
target_topology:
  - Oteryn
  - Oteryn-Game
  - Oteryn-Platform
  - Oteryn-Atlas
known_migration_sources:
  - blakinio/Oteryn-v2
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
legacy_read_only_sources:
  - blakinio/canary
  - blakinio/otclient
wave_1_evidence:
  implementation_pr: 1131
  implementation_merge: 43ceb7d17054787698c879a0797718e4a1cb1c28
  readiness_report: docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  coordinate_inventory: docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  atlas_extraction_manifest: docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  archived_task: docs/agents/tasks/archive/OTERYN-20260817-ecosystem-repository-migration-wave1.md
organization_access_evidence:
  owner_reported_target: Oteryn
  owner_reported_url: https://github.com/Oteryn/
  owner_reported_state: CREATED
  connector_user_orgs: []
  connector_org_memberships: []
  connector_installations:
    - installation_id: 78758924
      account_login: blakinio
      account_type: User
    - installation_id: 154585379
      account_login: Oteryn
      account_type: Organization
  connector_oteryn_visible: true
  organization_repository_probe:
    repository: Oteryn/Oteryn-Atlas
    visibility: public
    permissions:
      admin: true
      maintain: true
      push: true
      pull: true
      triage: true
  classification: PROVEN_ORGANIZATION_INTEGRATION_ACCESS
observed_target_coordinates:
  Oteryn:
    repository: Oteryn/Oteryn
    state: ABSENT_404
  Oteryn-Atlas:
    repository: Oteryn/Oteryn-Atlas
    state: EXISTS
    visibility: public
    size: 0
proven:
  - The programme alias is OTERYN-REPO-MIGRATION and resolves to the canonical prompt in this repository.
  - Wave 1 reconstructed live source/target state and implementation PR 1131 squash-merged through protected main as 43ceb7d17054787698c879a0797718e4a1cb1c28.
  - The owner reports that the intended GitHub organization is `Oteryn` and has been created.
  - The authenticated GitHub integration now exposes installation 154585379 for organization `Oteryn`.
  - Existing organization repository `Oteryn/Oteryn-Atlas` is visible through that integration with admin/maintain/push/pull/triage permission.
  - The intended META coordinate `Oteryn/Oteryn` currently returns 404 Not Found.
  - Existing `Oteryn/Oteryn-Atlas` is a live target repository and is disjoint from the independent META-creation transaction.
  - PR 1143 is the current durable organization-access task/branch checkpoint.
  - Oteryn-v2 package inventory and exhaustive external Actions/reusable-workflow caller evidence remain unresolved Game-specific blockers and were not refreshed because the current trusted Platform invocation does not authorize server/game repository inspection.
derived:
  - The accepted four-repository architecture remains valid.
  - The organization naming/access blocker is resolved; empty membership-list endpoints do not override direct installation and repository-permission proof.
  - A separate META repository-creation/bootstrap transaction for `Oteryn/Oteryn` is now the highest-value disjoint READY phase.
  - Game cutover and Atlas extraction remain independently fail-closed on their own blockers and must not be bundled with META creation.
unknown:
  - Whether installation 154585379 is configured for all repositories or selected repositories; resulting-state access must be verified after any new repository creation.
  - Exact organization-level repository-creation policy as exposed to this connector; the available GitHub tool set currently exposes no create-repository action.
  - Exhaustive external Actions/reusable-workflow callers of Oteryn-v2.
  - Exact Oteryn-v2 GHCR/package names, links, permissions and consumers.
  - Complete path-level Atlas ownership split needed for selective extraction.
conflicts: []
blockers:
  - Game-specific package/caller evidence remains unresolved for any future Oteryn-v2/Oteryn-Game physical cutover.
  - Atlas extraction remains separately coupled to source ownership/deployment evidence and must not be inferred ready from the existence of Oteryn/Oteryn-Atlas.
next_action: Complete PR 1143 organization-access closeout, then create one bounded META creation/bootstrap preparation task for Oteryn/Oteryn without accessing server/game repositories.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let the Game->Atlas spatial-profile evidence gap block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META authority supersedes Platform ADR 0041 only after the META authority is actually canonical.
- Every physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.
