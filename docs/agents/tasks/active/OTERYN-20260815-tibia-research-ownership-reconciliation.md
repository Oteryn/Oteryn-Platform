---
task_id: OTERYN-20260815-tibia-research-ownership-reconciliation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - PR #1006
  - issue #987
  - Oteryn-v2 PR #283
  - Oteryn-v2 PR #284
status: waiting
---

# OTERYN-20260815 Tibia research ownership reconciliation

## Goal

Reconcile stale Tibia/official-client research from Platform PRs #988 and #1006 against the accepted repository topology, preserve durable outputs on clean canonical delivery paths, and close historical branches only when repository and runtime closeout gates are satisfied.

## Delivery state

The repository-ownership migration is complete. One runtime closeout gate remains for source PR #1006, so this task is intentionally `waiting` rather than falsely `completed`.

## Acceptance criteria

- [x] Current ownership is reconciled against Platform ADR 0041 and Oteryn-v2 ADR-0002.
- [x] `blakinio/otclient` is classified historical/non-canonical for new Oteryn v2 client work.
- [x] Reusable #988 Platform infrastructure was hardened and merged through PR #1104 as `c014fcad498e2568cc47b64f6d886967f270d7a1`.
- [x] #1006 proprietary-data-free worldmap/client-world tooling was migrated through Oteryn-v2 PR #283 as `0c307db73832b824ccf50801e626671e0aeb38d1` and lifecycle-closeout PR #284 as `5d40711074dd914e0fcf8a95954180d84feef5f3`.
- [x] #988 was closed without merge with `Branch-Disposition: delete`; its source branch was verified absent. Issue #987 remains open/blocked for the still-unperformed dedicated-host offline validation.
- [x] #1006 live/debug/VNC/gdb/credential/private-message/screenshot/proprietary material was not promoted into Platform `main` or Oteryn-v2.
- [x] A task-owned cleanup attempt for #1006 failed closed before mutation when the client local SOCKS transport reappeared.
- [ ] #1006 task-owned Synology runtime resources are removed only after a fresh read-only check proves the client transport is gone, and canonical `oteryn-staging` invariance is verified.
- [ ] #1006 then receives an explicit terminal branch disposition, closes without wholesale merge, and its source ref is removed.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
modules:
  - lifecycle-reconciliation
  - execution-resource-closeout
dependencies:
  - Platform PR #1006 live runtime closeout
blockers:
  - PR #1006 task-owned client process can retain/re-establish a local SOCKS transport, so destructive cleanup is not currently safe
cross_repository_tasks:
  - none; Oteryn-v2 #283/#284 are terminal merged
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T00:18:00+02:00
head: none
branch: none
pr: none
status: waiting
context_routes:
  - agent-governance
  - execution-resource-hygiene
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
proven:
  - Platform PR #1104 final head f4ecbe1bfdd7a51940901d1dd236cdb968da1d44 passed CI 31911457035, Phase 7 31911457015, Platform DB Outage 31911457054, Agent Governance 31911457030 and Tibia Linux Reference Harness 31911457036 after all review findings were repaired; it merged as c014fcad498e2568cc47b64f6d886967f270d7a1 and its delivery branch was automatically removed.
  - Oteryn-v2 PR #283 merged the worldmap reference package as 0c307db73832b824ccf50801e626671e0aeb38d1; closeout PR #284 merged as 5d40711074dd914e0fcf8a95954180d84feef5f3, the active task is absent on Oteryn-v2 main and the archive record is present.
  - Platform PR #988 was closed without merge after adding Branch-Disposition: delete; branch search then returned no research/OTERYN-20260811-official-linux-offline-launch ref. Issue #987 remains open/blocked, so no false offline-execution completion was claimed.
  - Ephemeral cleanup-attempt PR #1105 closed without merge with Branch-Disposition: delete and its source branch was verified absent.
  - Historical #1006 session-check rerun job 95075794423 observed CLIENT_PID_PRESENT=true with zero local/direct TCP at one point.
  - Immediately following terminal-cleanup run 31911054031 job 95076020397 observed CLIENT_PID_PRESENT=true, ACTIVE_LOCAL_SOCKS_COUNT=1 and ACTIVE_DIRECT_TCP_COUNT=0; the destructive cleanup and verification steps were skipped by the fail-closed precondition.
  - PR #1006 now contains durable comment 5304478532 recording the canonical Oteryn-v2 destination, Platform #1104 delivery, runtime blocker and exact next terminal action.
derived:
  - Repository ownership is no longer ambiguous: new native client/game/world work belongs to Oteryn-v2, not historical blakinio/otclient.
  - #1006 must remain open until runtime cleanup is safe; deleting its branch now would violate execution-resource and source-branch closeout policy.
unknown:
  - When the local SOCKS transport will stop and remain absent long enough for immediate ownership-scoped cleanup.
conflicts: []
first_failure:
  marker: source-pr-1006-runtime-cleanup-blocked-by-live-transport
  evidence: cleanup run 31911054031 job 95076020397 failed before mutation because ACTIVE_LOCAL_SOCKS_COUNT=1
rejected_hypotheses:
  - A single zero-socket observation is sufficient to delete #1006 runtime resources; the immediately following cleanup gate disproved this.
  - Closing #1006 now is harmless because its durable code is already migrated; repository policy also requires safe cleanup of task-owned execution resources before terminal closeout.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
validation:
  - command: Oteryn-v2 delivery and lifecycle verification
    result: PASS
    evidence: main 5d40711074dd914e0fcf8a95954180d84feef5f3 contains terminal archive; implementation and closeout branches are absent
  - command: Platform clean ownership harvest
    result: PASS
    evidence: PR #1104 merged as c014fcad498e2568cc47b64f6d886967f270d7a1 after exact-head CI and review PASS
  - command: #988 stale branch lifecycle
    result: PASS
    evidence: PR #988 closed unmerged with delete disposition and source branch verified absent; Issue #987 remains blocked
  - command: #1006 task-owned runtime cleanup
    result: BLOCKED
    evidence: run 31911054031 failed closed before mutation because local SOCKS transport was active
blockers:
  - A fresh read-only no-session check must prove the #1006 client has no local SOCKS or direct TCP, followed immediately by ownership-scoped cleanup before the transport can reconnect.
next_action: Re-run the read-only #1006 session check; only if CLIENT_PID/transport evidence proves no active client session, immediately remove the exact task-owned container/bind/verified relay with canonical oteryn-staging before/after invariance checks, then close #1006 with Branch-Disposition: delete and verify source-ref removal.
```

## Waiting semantics

No worker should poll or delete resources while the transport remains live. The next invocation starts from the single `next_action` above; it must fail closed again if the client reconnects.
