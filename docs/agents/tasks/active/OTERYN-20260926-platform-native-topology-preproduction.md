---
task_id: OTERYN-20260926-platform-native-topology-preproduction
governing_issue: 1416
required_reads:
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
search_first:
  - native topology issuance and immutable committed readback
  - World-to-Channel ownership and existing integer compatibility
  - independent-process MariaDB issuance and retained rollback evidence
optional_reads: []
---

# OTERYN-20260926 Platform native topology preproduction

## Goal

Governing GitHub Issue: [#1416](https://github.com/Oteryn/Oteryn-Platform/issues/1416).

Implement a genuine Platform Registry-owned canonical WorldId + distinct ChannelId issuer/readback for one explicitly provisioned disposable preproduction room. Preserve old integer compatibility and leave production, deployment, credentials, backfill and route/readiness activation outside this task.

The owner-authorized lease is [#1416 comment5846491253](https://github.com/Oteryn/Oteryn-Platform/issues/1416#issuecomment-5846491253), expanded by [required diagnostic repair5846770645](https://github.com/Oteryn/Oteryn-Platform/issues/1416#issuecomment-5846770645). Canonical draft PR is [#1417](https://github.com/Oteryn/Oteryn-Platform/pull/1417). This is the repaired AUTHORING snapshot; live GitHub controls the successor freeze, qualification and closeout.

## Acceptance criteria

- [ ] Nullable canonical World UUIDv7 and first-class scoped Channel persist without defaults, seeds or backfill.
- [ ] Private trusted issuer commits before readback; exact replay/restart retains IDs and failed transactions emit no receipt.
- [ ] Ordinary model instance mutation and migration rollback retain issued identity, including inactive and stale-object cases.
- [ ] Existing integer World Registry/Ensure/protocol-candidate behavior remains compatible.
- [ ] Strict UUID/selector decoding and closed environment/database/transaction guards fail closed.
- [ ] Feature, regular-file reconnect, migration-retention and actual independent-process MariaDB race tests pass on the frozen candidate.
- [ ] Exact-candidate required CI and independent review complete before integration readiness.
- [ ] Protected integration and intentional source-branch/task archival are verified.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/Worlds/GameWorld.php
  - database/migrations/2026_09_26_150000_add_native_world_topology.php
  - app/GameAuth/Worlds/GameChannel.php
  - app/GameAuth/Worlds/NativeTopologyReceipt.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/IssueNativeTopology.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyRegistryTest.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyConcurrencyTest.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyMigrationRollbackTest.php
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/ci.yml
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260926-platform-native-topology-preproduction.md
modules:
  - GameAuth/Worlds
  - private disposable topology provisioning
dependencies:
  - Platform main@c54db8786cdef421ae3c378cbe50e564fbf597b7
  - ADR0029 and accepted World topology contract
blockers:
  - none for bounded authoring
cross_repository_tasks:
  - Game consumer qualification is separate; no Game product write under this task
```

The exact NativeTopology test subtree is separately allocated against the terminal historical broad #1388 test lease. NativeEvidence tests/code and the #1388 task record are unchanged.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-26T14:20:00Z
head: 063d7fed4e8b54b428259d7b352b7714a754af30
branch: codex/platform-native-topology-preproduction
pr: 1417
status: implementing
context_routes:
  - architecture
  - database
  - testing
owned_paths:
  - app/GameAuth/Worlds/GameWorld.php
  - database/migrations/2026_09_26_150000_add_native_world_topology.php
  - app/GameAuth/Worlds/GameChannel.php
  - app/GameAuth/Worlds/NativeTopologyReceipt.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/IssueNativeTopology.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyRegistryTest.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyConcurrencyTest.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyMigrationRollbackTest.php
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/ci.yml
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260926-platform-native-topology-preproduction.md
proven:
  - Owner-authorized bounded Platform scope and exclusive staged authoring lease exist.
  - Game FND-ID and Platform ADR0029 agree on distinct full-128-bit UUIDv7 domains.
  - composer.lock pins Laravel v13.30.1 and Ramsey UUID 4.9.3; exact upstream Str::uuid7 is available.
  - Existing SQLite feature and MariaDB11.8 pcntl CI routes are registered.
derived:
  - World row locking and unique scoped keys provide the bounded issuance serialization boundary.
unknown:
  - Successor compiler, format, static analysis, feature, MariaDB and aggregate CI results are pending; prior candidate failed qualification.
  - Required independent review and protected integration are pending.
conflicts: []
first_failure:
  marker: WORKFLOW_FAILURE
  evidence: Candidate063d7fed passed Pint and actual waiter observation, then failed8PHPStan type/assertion errors and a different-World Channel insert with real1213deadlock. Repair uses guarded row/status types and removes only redundant Channel range locking after World ownership; oracle/deadline unchanged.
rejected_hypotheses:
  - Integer World or channel1 is canonical native identity.
  - APP_ENV or UUID time proves issuer custody, readiness or current writer authority.
changed_paths:
  - app/GameAuth/Worlds/GameWorld.php
  - database/migrations/2026_09_26_150000_add_native_world_topology.php
  - app/GameAuth/Worlds/GameChannel.php
  - app/GameAuth/Worlds/NativeTopologyReceipt.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/IssueNativeTopology.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyRegistryTest.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyConcurrencyTest.php
  - tests/Feature/GameAuth/NativeTopology/NativeTopologyMigrationRollbackTest.php
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/ci.yml
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260926-platform-native-topology-preproduction.md
validation:
  - command: exact upstream Str::uuid7 source inspection against composer.lock
    result: PASS
    evidence: Laravel framework@718d17db56861e0a49f644217c8853dab1bff8ce delegates uuid7 to pinned Ramsey Uuid::uuid7.
  - command: local PHP compiler and runtime tests
    result: BLOCKED
    evidence: PHP executable is unavailable in the current Windows worker; no local execution PASS claimed.
  - command: frozen candidate repository CI and independent review
    result: NOT_RUN
    evidence: successor candidate is not yet frozen; exact candidate-specific evidence will follow publication.
  - command: prior candidate CI36247204364 and concurrency36247204340
    result: FAIL
    evidence: Pintpasses; PHPStan8actual errors confined to ConcurrencyTest, fullFeature notrun. Actualtwo-waiter oracle reached; two same-World cases finish without assertion failure/warningspendingdisplay; distinctWorlds emits1213deadlock on Channelinsert.
  - command: candidate063d7fed independent deep source review and AgentGovernance36247204341
    result: PASS
    evidence: Full13files/exact063d7fed reviewed; metadata cadence supported by exactupstream, no new material finding. CanonicalPR governance passes. SourcePASS does not replace failed runtime gates or qualify successor.
  - command: owning World serialization and first consistent read inspection
    result: PASS
    evidence: PinnedLaravel uses own normal PDO transaction/samewritePDO. ExactMariaDBbf9193 row0sel.cc4727–4728 opens view only for LOCK_NONE; lock0lock.cc6645–6647 checks snapshot only ifview alreadyopen. FirstdatareadWorldFORUPDATE creates no earlier snapshot; Channelconsistentread occurs after World ownership and sees predecessorcommit. Soleordinaryissuer/immutablemodelguards/uniqueFK remain. No isolation config/retryloop needed. Successor realtests still required.
blockers:
  - none for authoring; root controls proven repository validation routes
next_action: Publish the complete owned tree through the selected control-plane route, freeze its exact returned head and qualify that candidate.
```

The head above is the verified authoring base, not a self-reference to a future commit. Postfreeze status belongs to live Issue/PR evidence; no status-only commit is required.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected PR and Merge Queue path
source_branch_evidence: pending protected integration and source-ref absence readback
```

## Notes

Migration rollback is externally exclusive/quiescent and refuses before DDL once identity is issued. Fixture destruction is explicitly separate. No whole-database restore, privileged raw SQL, production/readiness or Game consumer authority is claimed. Root owns publication, independent review, qualification/MQ and terminal active-packet archival. Repair preserves actual2waiters/10secdeadline, bounded diagnostics and guaranteed cleanup. Mandatory Pint adds verbose output; native concurrency displays warnings. ActualMariaDB11.8.9@bf9193a939f515e95dd8def1a5468088c91cede6 trx0i_s.cc958–970/1069–1082/1129–1133/handleri_s.cc183/373/825 proves metadata idle>100ms; cadence150ms resolves observation. Later actual1213deadlock at Channelinsert motivates removing only redundant absent-Channel range lock. Exact[read-view source](https://github.com/MariaDB/server/blob/bf9193a939f515e95dd8def1a5468088c91cede6/storage/innobase/row/row0sel.cc#L4727) and[snapshot guard](https://github.com/MariaDB/server/blob/bf9193a939f515e95dd8def1a5468088c91cede6/storage/innobase/lock/lock0lock.cc#L6645) prove firstlockingWorldread creates no staleview. World ownership acquired before firstconsistentChannelread serializes sameWorld producers; independentWorlds need no Channelrange lock. Readback/UUIDguards/uniqueFK remain. The precise deadlockcycle is inferred, not an observedLATESTDEADLOCK report; actualsuccessor concurrency must prove repair. No isolationchange, retryloop, producttimer or permissionchange. FullCI/review pending.
