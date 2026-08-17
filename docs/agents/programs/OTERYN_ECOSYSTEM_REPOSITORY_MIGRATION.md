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
updated_at: 2026-08-17T12:42:00Z
status: blocked
active_task: OTERYN-20260817-ecosystem-repository-migration-wave1
issue: 1130
branch: docs/oteryn-ecosystem-repository-migration-wave1
pull_request: null
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
  readiness_report: docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  coordinate_inventory: docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  atlas_extraction_manifest: docs/architecture/migration/oteryn-atlas-extraction-manifest.json
proven:
  - The programme alias is OTERYN-REPO-MIGRATION and resolves to the canonical prompt in this repository.
  - Wave 1 reconstructed live source/target state and found no current target repositories at the inspected same-user names.
  - The authenticated GitHub account currently exposes no organization membership to the connector.
  - Game repository-local control-plane inspection is mostly rename-dynamic, but package inventory is inaccessible and external Actions/reusable-workflow callers are not exhaustively proven.
  - Current Otheryn Atlas automation includes an active private Synology deployment path and mixed build/deployment ownership.
derived:
  - The accepted four-repository architecture remains valid.
  - Physical META bootstrap, Game cutover and Atlas extraction remain fail-closed until the Wave-1 blockers are resolved.
unknown:
  - Exact future Oteryn GitHub organization identity/permissions.
  - Exhaustive external Actions/reusable-workflow callers of Oteryn-v2.
  - Exact Oteryn-v2 GHCR/package names, links, permissions and consumers.
  - Complete path-level Atlas ownership split needed for selective extraction.
conflicts: []
blockers:
  - Future Oteryn GitHub organization is not visible/available to the authenticated account.
  - Oteryn-v2 GHCR/package inventory is unavailable through the current integration.
  - External Actions/reusable-workflow caller inventory is not exhaustive and GitHub rename redirects do not protect this execution path.
  - Otheryn Atlas remains coupled to active private Synology deployment and mixed path ownership.
next_action: After Wave-1 evidence is merged and archived, create or identify the intended future Oteryn GitHub organization and make it visible to the authenticated GitHub account.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let the Game->Atlas spatial-profile evidence gap block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META authority supersedes Platform ADR 0041 only after the META authority is actually canonical.
- Every physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.