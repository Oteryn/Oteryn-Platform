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
status: validating
---

# OTERYN-20260815 Tibia research ownership reconciliation

## Goal

Reconcile stale Tibia/official-client research from Platform PRs #988 and #1006 against the accepted repository topology, preserve only durable Platform-owned infrastructure tooling on a clean current-main branch, migrate client/world semantic tooling to the canonical Oteryn-v2 lineage, and make the two historical research PRs eligible for intentional close/delete rather than blind merge.

## Acceptance criteria

- [x] Current ownership is reconciled against Platform ADR 0041 and Oteryn-v2 ADR-0002.
- [x] `blakinio/otclient` is classified historical/non-canonical for new Oteryn v2 client work.
- [x] The safe, reusable #988 official Linux identity/host/evidence tooling is harvested onto current Platform `main` lineage without the blocked task state or proprietary artifacts.
- [x] #1006 worldmap reconstruction tooling and client/game semantic evidence are routed to Oteryn-v2 PR #283.
- [x] Branch-only #1006 diagnostic/live workflows, credentials paths, screenshots/base64 evidence, VNC and gdb/ptrace scaffolding are excluded from Platform `main` and from the destination migration.
- [ ] Focused harvested-tool validation and Platform required exact-head CI pass.
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
  - .github/workflows/tibia-linux-live-reference.yml
modules:
  - external-reference-infrastructure
  - agent-governance
dependencies:
  - Platform PR #988
  - Platform PR #1006
  - Oteryn-v2 PR #283
blockers: []
cross_repository_tasks:
  - blakinio/Oteryn-v2#283
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T23:44:00+02:00
head: 61dfb7a0bf0c322a92b663de8c8ca4ba26134306
branch: chore/OTERYN-20260815-tibia-research-ownership-reconciliation
pr: 1104
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
  - .github/workflows/tibia-linux-live-reference.yml
proven:
  - Platform ADR 0041 assigns native client/server/protocol/world/content ownership to the Oteryn-Game lineage and identifies blakinio/Oteryn-v2 as its current source lineage.
  - Oteryn-v2 ADR-0002 makes blakinio/Oteryn-v2 canonical for the native Rust client and makes blakinio/otclient historical migration/reference evidence after cutover.
  - Platform PR #988 head f9ff34b37cf81c400a48f7ab9329393416ac304d contains reusable non-executing official Linux identity, host-preflight, dedicated-user and encrypted-evidence tooling while its dedicated-host execution acceptance remains blocked.
  - Platform PR #1006 head 97f8df9e64e1e4f0520440073e497f24dad929ef contains a six-file proprietary-data-free worldmap reconstruction package plus many experimental live-client workflows that are not suitable for wholesale merge.
  - Oteryn-v2 PR #283 current head 54165596f98b66c3164cccab881bb53f0655cb2b contains the migrated worldmap package and repairs all four initial automated-review findings with resolved review threads.
  - Platform PR #1104 initial focused validate-tooling job 95073402601 passed the harvested official Linux tooling tests.
derived:
  - Neither Platform PR #988 nor #1006 should be merged wholesale into Platform main.
  - The existing registered tibia-linux-live-reference workflow should be extended instead of introducing a new task-specific validation workflow.
unknown:
  - Final exact-head CI and merge result for Platform PR #1104 after lifecycle/checkpoint repair.
  - Final exact-head CI and merge result for Oteryn-v2 PR #283 after review repair and PR-metadata correction.
conflicts: []
first_failure:
  marker: initial-harvest-governance-and-workflow-lifecycle-contract
  evidence: Platform CI run 31909950063 job 95073402356 failed workflow inventory because a new unregistered task-specific workflow was added; Agent Governance run 31909950113 job 95073402325 failed because the initial task checkpoint omitted required contract fields and live PR identity.
rejected_hypotheses:
  - Register a second durable Tibia Linux validation workflow merely for the #988 harvest; repository lifecycle policy requires reuse or extension when the existing tibia-linux-live-reference workflow can prove the same bounded tooling.
  - Merge source PR #1006 to preserve its worldmap package; its live-client diagnostic workflows are execution history and the durable package is already routed through Oteryn-v2 PR #283.
changed_paths:
  - .github/workflows/tibia-linux-live-reference.yml
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
validation:
  - command: Platform PR 1104 initial validate-tooling run 31909950097 job 95073402601
    result: PASS
    evidence: harvested identity, preflight and shell-safety tests passed without official binary execution or proprietary artifacts.
  - command: Platform PR 1104 initial CI run 31909950063 job 95073402356
    result: FAIL
    evidence: workflow inventory rejected the newly introduced unregistered task-specific workflow before path classification.
  - command: Platform PR 1104 initial Agent Governance run 31909950113 job 95073402325
    result: FAIL
    evidence: initial active task checkpoint omitted required version-1 contract fields and live PR identity; subsequent Control Room/liveness steps therefore also failed.
  - command: Oteryn-v2 PR 283 focused worldmap-reconstruction run 31910093544 job 95073747757
    result: PASS
    evidence: repaired worldmap tool compile, unit and synthetic CLI validation passed on exact head 54165596f98b66c3164cccab881bb53f0655cb2b.
blockers: []
next_action: Reuse the registered tibia-linux-live-reference workflow and corrected checkpoint contract on PR #1104, require fresh exact-head CI on both clean destination PRs, then merge them before closing source PRs #988 and #1006.
```
