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
updated_at: 2026-08-18T05:14:18Z
status: blocked
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
  connector_oteryn_visible: false
  classification: BLOCKED_ON_INTEGRATION_VISIBILITY
proven:
  - The programme alias is OTERYN-REPO-MIGRATION and resolves to the canonical prompt in this repository.
  - Wave 1 reconstructed live source/target state and found no current target repositories at the inspected same-user names.
  - Wave 1 implementation PR 1131 passed exact-head repository-selected validation and squash-merged through protected main as 43ceb7d17054787698c879a0797718e4a1cb1c28.
  - The authenticated GitHub account exposed no organization membership to the connector during Wave 1.
  - Game repository-local control-plane inspection was mostly rename-dynamic, but package inventory remained inaccessible and external Actions/reusable-workflow callers were not exhaustively proven.
  - Current Otheryn Atlas automation included an active private Synology deployment path and mixed build/deployment ownership at the Wave 1 observation baseline.
  - The owner reports that the intended GitHub organization is now `Oteryn` and has been created.
  - On 2026-08-18 the current authenticated GitHub connector still returned no user organizations, no organization memberships and only installation 78758924 for account `blakinio`.
  - Draft PR 1143 is the current durable task/branch checkpoint for organization-access recovery.
derived:
  - The accepted four-repository architecture remains valid.
  - Organization creation resolves the naming/owner-choice part of the prior blocker but does not prove current connector access or organization repository permissions.
  - Physical META bootstrap, Game cutover and Atlas extraction remain fail-closed until their independent blockers are resolved.
unknown:
  - Whether the current authenticated GitHub identity is a member/owner of `Oteryn` from the connector perspective.
  - Whether the ChatGPT GitHub App is installed/authorized for `Oteryn`.
  - Exact organization-level repository creation/transfer permissions exposed after connector authorization.
  - Exhaustive external Actions/reusable-workflow callers of Oteryn-v2.
  - Exact Oteryn-v2 GHCR/package names, links, permissions and consumers.
  - Complete path-level Atlas ownership split needed for selective extraction.
conflicts: []
blockers:
  - `Oteryn` is not visible to the current authenticated GitHub integration: organization and membership lists are empty and no Oteryn app installation is exposed.
  - Oteryn-v2 GHCR/package inventory is unavailable through the current integration.
  - External Actions/reusable-workflow caller inventory is not exhaustive and GitHub rename redirects do not protect this execution path.
  - Otheryn Atlas remains coupled to active private Synology deployment and mixed path ownership.
next_action: Install or authorize the ChatGPT GitHub integration for the `Oteryn` organization, then rerun organization membership/installation visibility before any repository creation, transfer or rename.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let the Game->Atlas spatial-profile evidence gap block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META authority supersedes Platform ADR 0041 only after the META authority is actually canonical.
- Every physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.