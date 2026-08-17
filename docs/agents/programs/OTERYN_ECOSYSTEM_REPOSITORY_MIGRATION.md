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
updated_at: 2026-08-17T08:55:00Z
status: ready
active_task: null
issue: null
branch: null
pull_request: null
temporary_topology_authority:
  repository: blakinio/Oteryn-Platform
  path: docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  status: VERIFY_LIVE_ON_INVOCATION
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
proven:
  - The programme alias is OTERYN-REPO-MIGRATION and resolves to the canonical prompt in this repository after this registration is merged.
  - Physical repository state, current SHAs, organizations, open ownership and migration readiness are intentionally not cached as truth here and must be reconstructed live on every invocation.
derived:
  - The first high-value migration increment is repository-coordinate and control-plane inventory before physical rename/transfer.
unknown:
  - Whether Oteryn, Oteryn-Game or Oteryn-Atlas already exist at invocation time.
  - Exact current GitHub organization/ownership target and whether META creation should occur before product-repository rename/transfer.
  - Exact current rename/transfer blockers in workflows, GHCR/packages, releases, provenance and cross-repository references.
conflicts: []
blockers: []
next_action: Run fresh live-state reconstruction, deduplicate existing migration ownership, and complete the migration-critical repository-coordinate inventory before selecting the first physical cutover increment.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let the Game->Atlas spatial-profile evidence gap block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META authority supersedes Platform ADR 0041 only after the META authority is actually canonical.
- Every physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.