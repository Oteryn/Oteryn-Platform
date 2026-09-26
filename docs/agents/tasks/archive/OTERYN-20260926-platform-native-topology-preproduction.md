---
task_id: OTERYN-20260926-platform-native-topology-preproduction
governing_issue: 1416
required_reads:
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
search_first:
  - committed native topology and retained identity
optional_reads: []
---

# OTERYN-20260926 Platform native topology preproduction

## Goal and historical outcome

Governing Issue: [#1416](https://github.com/Oteryn/Oteryn-Platform/issues/1416). Code PR [#1417](https://github.com/Oteryn/Oteryn-Platform/pull/1417) is integrated. This archived packet records verified code delivery; its own proposed archival integration is resolved by the live Issue closeout, without a future self-referential SHA.

Delivered the owner-authorized private disposable Registry issuer for canonical UUIDv7 WorldId and distinct scoped ChannelId, committed readback, explicit command, retained identity and integer compatibility. No deployment, production data, backfill, credentials, readiness, native routing or Game assignment was authorized or performed. Game coordination belongs to the owner-selected Claude control plane; this is the separate standalone Platform task.

## Acceptance criteria

- [x] Nullable canonical World storage and first-class scoped Channel, without defaults/seeds/backfill.
- [x] Private issuance and replay retain identity, commit before readback, refuse outer transactions and emit no failed receipt.
- [x] Ordinary model events and migration rollback retain issued identity, including inactive and stale-object cases.
- [x] Strict selectors/UUIDs and closed disposable environment/database/file guards fail closed; guards are not custody proof.
- [x] Existing integer Registry/Ensure/protocol-candidate compatibility retained.
- [x] Feature, retained-file reconnect, migration and three actual independent-process MariaDB cases qualified.
- [x] Complete candidate checks, independent material review and protected code integration verified.
- [x] Original source branch auto-deletion verified.
- [ ] Protected integration of this archival proposal and final Issue closeout: pending at document authoring; see live #1416 for terminal receipt.

## Ownership

Historical implementation scope is the exact thirteen files of PR1417. Root owned its branch and publication; subordinate authors stopped before publication. Closeout lease [5847081438](https://github.com/Oteryn/Oteryn-Platform/issues/1416#issuecomment-5847081438) authorizes only the active/archive task paths below on a separate root-owned branch. No implementation, contract, workflow or producer bytes change in closeout.

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260926-platform-native-topology-preproduction.md
  - docs/agents/tasks/archive/OTERYN-20260926-platform-native-topology-preproduction.md
modules:
  - task lifecycle closeout
dependencies:
  - protected Platform code integration eb65ce453949a352fdc0e3ed0132d2cde0084fb7
blockers:
  - none for bounded archival authoring
cross_repository_tasks:
  - Game consumption remains separately allocated by its selected control plane
```

## Context checkpoint

This is a historical implementation checkpoint, with the contract's explicit archive-pending terminal transition. Exact later archival candidate and readback belong to the live Issue/PR.

```yaml
checkpoint_version: 1
updated_at: 2026-09-26T14:29:00Z
head: 3330bdabe5d6928f519133132ca933e7d8cbf626
branch: codex/platform-native-topology-preproduction
pr: 1417
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - architecture
  - database
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260926-platform-native-topology-preproduction.md
  - docs/agents/tasks/archive/OTERYN-20260926-platform-native-topology-preproduction.md
proven:
  - Owner-authorized disposable scope and exclusive source leases were reconciled before writes.
  - Frozen3330bd candidate contains the complete thirteen-file delta; all intended blobs were verified on protected main.
  - All twelve selected source workflows succeeded; source checks30SUCCESS/2trusted-unselectedSKIPPED.
  - Actual native process qualification completed all three two-waiter/product cases with108assertions.
  - Protected code merge eb65ce453949a352fdc0e3ed0132d2cde0084fb7 and source branch absence are verified.
derived:
  - World ownership before the first consistent Channel read serializes the sole ordinary issuers for that World.
unknown:
  - Protected archival receipt and final Issue closure are pending this author's snapshot; live Issue governs them.
conflicts: []
first_failure:
  marker: none
  evidence: Earlier concrete format/type/metadata/deadlock failures were repaired through new frozen candidates and qualified before integration.
rejected_hypotheses:
  - Integer World/channel1 is canonical native identity.
  - APP_ENV, UUID time or copied receipt establishes custody, assignment or current readiness.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260926-platform-native-topology-preproduction.md
  - docs/agents/tasks/archive/OTERYN-20260926-platform-native-topology-preproduction.md
validation:
  - command: candidate integrity and independent whole-material-source review
    result: PASS
    evidence: Freeze5846978032 and PR1417comment5847008187 bind3330bdabe5d6928f519133132ca933e7d8cbf626; full13files1356additions/5deletions, allblobs, no confirmedP0/P1/newP2.
  - command: compiler style static analysis and full Feature route
    result: PASS
    evidence: SourceCI36247920728/runtime108420470709/platform-gate108420820475;5700assertions, no failed tests. Existing optionalDotenv .env warnings remain reported/unsuppressed, not rewritten as warning-free.
  - command: actual independent-process MariaDB qualification
    result: PASS
    evidence: Run36247920789/job108420471984 succeeds;3actualtwo-waiter/finalproductcases108assertions. Existing bootstrapwarnings displayed. Retained-file and migration cases are on fullFeature route.
  - command: all selected source checks
    result: PASS
    evidence: READY5847037503; all12workflows including native audits/governance/CodeQL/outage/Phase7/PortalAcceptance/fullE2E succeeded before queue.
  - command: native protected integration and terminal blob readback
    result: PASS
    evidence: META36248449673/command5847037602/UUIDd0a21e83-88b5-474b-9495-ae675b9230ee/sequence1and2; actualmerge_group36248479164/platform-gate108422349934SUCCESS; PRmerged14:28:43Z; protectedeb65ce453949a352fdc0e3ed0132d2cde0084fb7 all13intendedblobsverified.
  - command: original source branch disposition
    result: PASS
    evidence: GitHubauto_delete_after_merge configured; codex/platform-native-topology-preproduction ref404 after verified merge.
  - command: this archive proposal qualification and protected readback
    result: NOT_RUN
    evidence: Authoring snapshot before its own commit/freeze; root must verify absence from active on protected main before closing1416.
blockers:
  - none for archival authoring
next_action: Archive this packet through the separately frozen protected closeout, verify the protected readback and close Issue1416.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected integration completed
source_branch_evidence: original source ref404 after PR1417 merged at eb65ce453949a352fdc0e3ed0132d2cde0084fb7
```

The separate closeout branch is `codex/platform-native-topology-closeout`, admitted at protected eb65ce453949a352fdc0e3ed0132d2cde0084fb7, with the same intentional auto-deletion requirement after its own integration. Jira synchronization is PENDING because no exact native-topology Story mapping was found. No duplicate programme item was created.

## Qualification limits and next consumer

Issuance is not Game assignment or live authority. The qualifying Game runner must pin the actually integrated producer, trusted isolated custody and committed readback, then independently qualify source/frame/activation/placement/controller/Recovery. Aggregate Game822 remains separate. Schema rollback is externally exclusive/quiescent, refuses before DDL once identity exists, and does not prove privileged raw-SQL or whole-snapshot restore safety. Retained fixture destruction is distinct from a product deletion API.
