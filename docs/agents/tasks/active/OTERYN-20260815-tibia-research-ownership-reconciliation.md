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
- [x] #1006 worldmap reconstruction tooling and client/game semantic evidence are routed to Oteryn-v2 and merged by PR #283 as `0c307db73832b824ccf50801e626671e0aeb38d1`.
- [x] Branch-only #1006 diagnostic/live workflows, credentials paths, screenshots/base64 evidence, VNC and gdb/ptrace scaffolding are excluded from Platform `main` and from the destination migration.
- [x] The harvested official-execution path now enforces dedicated-host/UID/graphics gates before delegating to the bounded launcher.
- [x] The destructive LUKS setup treats `wipefs` inspection failure as a hard stop before formatting.
- [ ] Fresh exact-head focused validation and Platform required CI pass after review repairs.
- [ ] Source PR #988 receives terminal delete disposition and closes only after this clean harvest is merged.
- [ ] Source PR #1006 remains open while its task-owned live SOCKS session prevents safe runtime cleanup; it must not be closed or deleted until cleanup is proven.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
  - tools/tibia-linux-reference/tibia_linux_reference/official_host.py
  - tools/tibia-linux-reference/tibia_linux_reference/official_execution.py
  - tools/tibia-linux-reference/tibia_linux_reference/cli.py
  - .github/workflows/tibia-linux-live-reference.yml
modules:
  - external-reference-infrastructure
  - agent-governance
dependencies:
  - Platform PR #988
  - Platform PR #1006
  - Oteryn-v2 PR #283 merged as 0c307db73832b824ccf50801e626671e0aeb38d1
blockers: []
cross_repository_tasks:
  - blakinio/Oteryn-v2#283 (merged)
  - blakinio/Oteryn-v2#284 (lifecycle closeout)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T00:11:00+02:00
head: pending-review-repair-commit
branch: chore/OTERYN-20260815-tibia-research-ownership-reconciliation
pr: 1104
status: validating
context_routes:
  - agent-governance
  - architecture
  - security
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - tools/tibia-linux-reference/**
  - .github/workflows/tibia-linux-live-reference.yml
proven:
  - Platform ADR 0041 assigns native client/server/protocol/world/content ownership to the Oteryn-Game lineage and identifies blakinio/Oteryn-v2 as its current source lineage.
  - Oteryn-v2 ADR-0002 makes blakinio/Oteryn-v2 canonical for the native Rust client and makes blakinio/otclient historical migration/reference evidence after cutover.
  - Oteryn-v2 PR #283 merged the proprietary-data-free worldmap package as main commit 0c307db73832b824ccf50801e626671e0aeb38d1 after four review findings were repaired and exact-head governance/reference/merge-gate checks passed.
  - Platform PR #988 contains reusable non-executing official Linux identity, host-preflight, dedicated-user and encrypted-evidence tooling while its dedicated-host execution acceptance remains blocked.
  - Platform PR #1006 is dominated by experimental live-client workflows and must not be merged wholesale.
  - Initial PR #1104 workflow-lifecycle/checkpoint failures were repaired by reusing the registered tibia-linux-live-reference workflow and correcting the v1 task record.
  - Full Platform CI then exposed an inherited ADR 0040 registry defect; b779d2d92532a562c845fd672885ef65211d35c9 added the required explicit supersession target and the fresh exact-head CI/Phase7/DB-outage/reference/governance generation passed.
  - Fresh review on #1104 found three remaining material safety gaps: wipefs inspection failure could fall through, the official execution CLI did not enforce the new dedicated-host/graphics checks, and getpass-based username resolution was environment-spoofable.
  - The repair introduces package-level official_host gates using os.getuid()+pwd.getpwuid, an official_execution wrapper used by the CLI before launcher delegation, fail-closed wipefs status handling, and focused regressions for all three findings.
  - A read-only rerun of the historical #1006 session check initially observed zero sockets, but a fail-closed terminal cleanup attempt one minute later observed ACTIVE_LOCAL_SOCKS_COUNT=1 and therefore skipped every destructive step; #1006 runtime cleanup remains blocked by a surviving/reconnecting local SOCKS session.
derived:
  - Neither Platform PR #988 nor #1006 should be merged wholesale into Platform main.
  - #1006 cannot yet receive a terminal branch disposition because live task-owned runtime resources cannot be safely removed while the client transport survives/reconnects.
unknown:
  - Final exact-head result of the review-repair generation for Platform PR #1104.
  - Exact future time when the #1006 local SOCKS session is definitively gone and ownership-scoped runtime cleanup can execute safely.
conflicts: []
first_failure:
  marker: pr-1104-review-safety-gaps
  evidence: three unresolved review threads identified wipefs fail-open behavior, unenforced official host gates in the CLI execution path, and environment-spoofable dedicated-user identity; all are repaired in the pending coherent commit and require fresh exact-head validation.
rejected_hypotheses:
  - Merge PR #1104 merely because the previous exact-head CI was green; open material review findings block merge.
  - Treat getpass.getuser() as proof of process identity; it trusts environment variables before the password database.
  - Treat a failing wipefs inspection as equivalent to a blank disk; destructive setup must fail closed.
  - Close #1006 after one no-socket observation; the immediately following cleanup gate proved the local SOCKS transport had reappeared.
changed_paths:
  - .github/workflows/tibia-linux-live-reference.yml
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
  - tools/tibia-linux-reference/tibia_linux_reference/official_host.py
  - tools/tibia-linux-reference/tibia_linux_reference/official_execution.py
  - tools/tibia-linux-reference/tibia_linux_reference/cli.py
validation:
  - command: Platform PR 1104 pre-review-repair exact-head generation on b779d2d92532a562c845fd672885ef65211d35c9
    result: PASS
    evidence: CI 31910900981, Phase 7 31910901013, DB Outage 31910900997, Agent Governance 31910900969 and Tibia Linux Reference Harness 31910900996 all passed after the inherited ADR repair.
  - command: review-repair focused tests and exact-head CI
    result: NOT_RUN
    evidence: coherent repair commit is being prepared.
blockers: []
next_action: Commit the three review repairs with regression coverage, resolve the review threads only after focused validation passes, run fresh exact-head CI, then merge PR #1104 if no new material finding remains.
```
