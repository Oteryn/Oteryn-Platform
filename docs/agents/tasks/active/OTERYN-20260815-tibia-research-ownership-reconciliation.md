---
task_id: OTERYN-20260815-tibia-research-ownership-reconciliation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - PR #988
  - PR #1006
  - Oteryn-v2 PR #283
---

# OTERYN-20260815 Tibia research ownership reconciliation

```yaml
status: validating
mode: implementation
repository: blakinio/Oteryn-Platform
branch: chore/OTERYN-20260815-tibia-research-ownership-reconciliation
pr: null
base_sha: 575c6f178414945cc109e12baeb588bbc3dfbc73
cross_repository_coordination_id: OTER-CLIENT-REFERENCE-HARVEST-20260815
external_repository: blakinio/Oteryn-v2
external_destination_pr: 283
```

## Goal

Reconcile stale Tibia/official-client research from Platform PRs #988 and #1006 against the accepted repository topology, preserve only durable Platform-owned infrastructure tooling on a clean current-main branch, migrate client/world semantic tooling to the canonical Oteryn-v2 lineage, and make the two historical research PRs eligible for intentional close/delete rather than blind merge.

## Acceptance criteria

- [x] Current ownership is reconciled against Platform ADR 0041 and Oteryn-v2 ADR-0002.
- [x] `blakinio/otclient` is classified historical/non-canonical for new Oteryn v2 client work.
- [x] The safe, reusable #988 official Linux identity/host/evidence tooling is harvested onto current Platform `main` lineage without the blocked task state or proprietary artifacts.
- [x] #1006 worldmap reconstruction tooling and client/game semantic evidence are routed to Oteryn-v2 PR #283.
- [x] Branch-only #1006 diagnostic/live workflows, credentials paths, screenshots/base64 evidence, VNC and gdb/ptrace scaffolding are excluded from Platform `main` and from the destination migration.
- [ ] Focused #988 tooling tests and Platform required exact-head CI pass.
- [ ] Oteryn-v2 PR #283 is merged or otherwise terminally preserves the destination work before #1006 closes.
- [ ] Source PRs #988 and #1006 receive explicit terminal branch dispositions and are intentionally closed without merge after durable harvest is verified.
- [ ] Source refs are removed through the repository branch-lifecycle control after closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
  - .github/workflows/tibia-linux-official-identity.yml
modules:
  - external-reference infrastructure
  - agent governance
cross_repository_tasks:
  - blakinio/Oteryn-v2 PR #283
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T23:35:00+02:00
status: validating
branch: chore/OTERYN-20260815-tibia-research-ownership-reconciliation
pr: null
head: null
proven:
  - Platform ADR 0041 assigns native client/server/protocol/world/content ownership to the Oteryn-Game lineage and makes blakinio/Oteryn-v2 the current target source lineage.
  - Oteryn-v2 ADR-0002 makes blakinio/Oteryn-v2 canonical for the native Rust client and makes blakinio/otclient historical migration/reference evidence after cutover.
  - PR #988 contains a bounded offline identity/host-isolation tooling package plus a blocked external-host research lifecycle; its reusable tooling is Platform infrastructure rather than native game implementation.
  - PR #1006 contains 76 changed files/302 commits dominated by experimental runner/live-client workflows; its six-file proprietary-data-free worldmap reconstruction package is the durable game/world artifact and is migrated by Oteryn-v2 PR #283.
  - Oteryn-v2 PR #283 was created from current main head 0cfd8d8ee3ecf4fbb1cb76cbc9680b53a152e3c1 and excludes live-client/credential/proprietary research scaffolding.
derived:
  - Neither #988 nor #1006 should be merged wholesale into Platform main.
  - Closing the source PRs is safe only after their intended durable outputs are present on validated clean delivery paths.
unknown:
  - final exact-head CI/merge results for this Platform harvest and Oteryn-v2 PR #283
conflicts: []
validation:
  - command: focused official Linux tooling workflow
    result: NOT_RUN
    evidence: pending PR creation
  - command: Platform required exact-head CI
    result: NOT_RUN
    evidence: pending PR creation
blockers: []
next_action: Open the clean Platform harvest PR, validate both destination PRs, merge them, then close #988/#1006 with Branch-Disposition: delete and invoke terminal branch lifecycle cleanup.
```
