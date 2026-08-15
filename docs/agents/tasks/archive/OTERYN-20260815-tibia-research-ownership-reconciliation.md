---
task_id: OTERYN-20260815-tibia-research-ownership-reconciliation
status: completed
project_lane: oteryn-platform-core
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
search_first:
  - PR #988
  - PR #1006
  - PR #1104
  - PR #1106
  - Oteryn-v2 PR #283
  - Oteryn-v2 PR #284
optional_reads: []
---

# OTERYN-20260815 Tibia research ownership reconciliation — terminal archive

## Terminal outcome

The stale Tibia/official-client research that had accumulated in Platform PRs #988 and #1006 was reconciled by responsibility rather than merged wholesale.

Current repository authority is explicit:

- native Oteryn client/game/protocol/world semantics belong to the Oteryn-Game lineage, currently `blakinio/Oteryn-v2`;
- `blakinio/otclient` is historical migration/reference evidence only and receives no new Oteryn v2 implementation work;
- Platform retains reusable host/control-plane reference infrastructure where that tooling is a Platform infrastructure concern rather than native game code.

The durable outputs are now on clean canonical delivery paths, both stale research PRs are closed without merge, their source refs are absent, and all task-owned Synology runtime resources from #1006 were removed with canonical `oteryn-staging` invariance proven.

## Delivery evidence

### Oteryn-v2 client/world harvest

The proprietary-data-free `tools/tibia-worldmap-reconstruction/**` package and durable client/world ownership evidence from Platform #1006 were migrated to `blakinio/Oteryn-v2`.

Oteryn-v2 PR #283 repaired four automated-review findings, passed exact-head merge/governance/reference validation and merged as:

`0c307db73832b824ccf50801e626671e0aeb38d1`

Lifecycle closeout PR #284 merged as:

`5d40711074dd914e0fcf8a95954180d84feef5f3`

The Oteryn-v2 active task is absent, its terminal archive is present, and both delivery/closeout branches are absent.

### Platform #988 reusable reference infrastructure

Reusable official archive identity, dedicated-host/real-UID/direct-rendering preflight, Ubuntu dedicated-user preparation, interactive LUKS2 evidence-volume setup and focused tests were harvested from historical PR #988 onto current Platform lineage.

Fresh review found and repaired three material safety defects before delivery:

- `wipefs` inspection failure now hard-stops before any LUKS formatting;
- the documented official execution CLI enforces dedicated-host/real-UID/accelerated-graphics gates before launcher delegation;
- dedicated-user identity is resolved from `os.getuid()` plus the password database rather than spoofable `USER`/`LOGNAME` environment variables.

Platform PR #1104 final head `f4ecbe1bfdd7a51940901d1dd236cdb968da1d44` passed:

- CI `31911457035`;
- Phase 7 Production-Like Validation `31911457015`;
- Platform DB Outage Validation `31911457054`;
- Agent Governance `31911457030`;
- Tibia Linux Reference Harness `31911457036`.

PR #1104 merged as `c014fcad498e2568cc47b64f6d886967f270d7a1` and its source branch was removed.

Historical PR #988 was then closed without merge with `Branch-Disposition: delete`; branch lookup returned no `research/OTERYN-20260811-official-linux-offline-launch` ref. Issue #987 intentionally remains open/blocked because dedicated-host official package acquisition/execution/BattlEye acceptance is still unperformed.

### Platform #1006 runtime and branch closeout

PR #1006 was not merged. Its 302-commit/76-file history is dominated by experimental live-client/login/VNC/private-message/gdb/ptrace/blind-movement workflows and diagnostics that are execution history rather than canonical product code.

A first cleanup attempt correctly failed closed when the client re-established one local SOCKS transport. No destructive step ran.

A later fresh read-only session rerun job `95077830819` proved:

```text
CLIENT_PID_PRESENT=true
ACTIVE_LOCAL_SOCKS_COUNT=0
ACTIVE_DIRECT_TCP_COUNT=0
```

The immediately-following ownership-scoped terminal cleanup run `31911829879`, job `95077983102`, revalidated the exact `oteryn-synology-staging` runner, task labels, task identity and exact `/volume1/docker/oteryn/tibia-analysis:/data` bind before mutation, and again proved zero local/direct client transport.

It then removed only the verified task-owned runtime resources:

- `oteryn-tibia-vnc-relay` if present;
- container `oteryn-tibia-client-analysis`;
- bind `/volume1/docker/oteryn/tibia-analysis`.

The bind size before removal was `1171069521` bytes.

Post-cleanup verification proved:

- task container absent;
- task relay absent;
- zero containers remain with task label `OTERYN-20260811-tibia-client-analysis`;
- bind absent;
- canonical `oteryn-staging` inventory unchanged before/after;
- `TASK_RUNTIME_CLEANUP=PASS`;
- `CANONICAL_OTERYN_STAGING_UNCHANGED=true`.

PR #1006 was then updated with `Branch-Disposition: delete`, closed without merge, and exact branch lookup returned no `ops/oteryn-tibia-client-analysis-20260811` ref.

The successful one-shot cleanup workflow branch was itself closed through PR #1107 with `Branch-Disposition: delete`; its exact source ref is also absent. The earlier failed-close cleanup branch/PR #1105 was likewise terminally deleted.

## Scope intentionally excluded from canonical delivery

No Platform branch-only live-client login/recovery orchestration, VNC, private-message action, gdb/ptrace/live-attach experiment, blind movement workflow, screenshot/base64 evidence, credential/session material or proprietary Tibia binary/asset was promoted into Platform `main` or Oteryn-v2.

The ownership reconciliation does not claim resolution of remaining research questions such as exact player XYZ structure, tile passability, raw Worldmap semantic mapping, higher-level native action ABI, full OTBM-recoverable coverage or current official Linux client/BattlEye offline behavior.

## Inherited Platform repair

During clean Platform delivery, exact-head CI exposed a pre-existing ADR registry defect: superseded ADR 0040 lacked the required explicit supersession target marker. PR #1104 added `- Superseded by: 0041-ecosystem-repository-authority-contracts-and-atlas-integration.md`. The resulting exact-head CI generation passed.

## Validation and review

- Oteryn-v2 #283 exact-final-head merge/governance/reference validation: PASS.
- Oteryn-v2 #284 lifecycle-only closeout validation: PASS.
- Platform #1104 exact-final-head CI/governance/reference validation: PASS.
- Platform #1104 whole-diff self-review: PASS on `f4ecbe1bfdd7a51940901d1dd236cdb968da1d44`.
- Platform #1104 material review threads: all repaired and resolved.
- Platform #1106 waiting-state checkpoint CI `31911755835`: PASS.
- Platform #1106 Agent Governance `31911755838`: PASS.
- #1006 runtime cleanup `31911829879` / `95077983102`: PASS with canonical staging unchanged.
- Runtime/browser E2E for this lifecycle-only archive transition: `NOT_APPLICABLE`; the material runtime cleanup is directly proven by the environment evidence above.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-16T00:24:00+02:00
head: 07a9faf909b85bfa6dadd71d5ba158af5ca688ff
branch: docs/OTERYN-20260815-tibia-research-closeout
pr: none
status: completed
context_routes:
  - agent-governance
  - architecture
  - execution-resource-hygiene
  - testing
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
proven:
  - new native client/game/world work belongs to Oteryn-v2 rather than historical blakinio/otclient
  - Oteryn-v2 #283/#284 are terminal merged and their task is archived
  - Platform #1104 is terminal merged as c014fcad498e2568cc47b64f6d886967f270d7a1 after exact-head CI and review PASS
  - Platform #988 is closed unmerged with delete disposition and its source ref is absent while Issue #987 remains blocked/open
  - #1006 cleanup run 31911829879 job 95077983102 passed and canonical oteryn-staging inventory was unchanged
  - Platform #1006 is closed unmerged with delete disposition and its source ref is absent
  - ephemeral cleanup branches from #1105 and #1107 are absent
  - no proprietary client binary/assets or credentials/session material were promoted to canonical delivery
unknown:
  - closeout PR number and closeout merge SHA until this lifecycle-only archive transition is delivered
  - closeout branch final absence until after closeout merge
conflicts: []
first_failure:
  marker: source-pr-1006-runtime-cleanup-initially-blocked-by-live-transport
  evidence: first cleanup run 31911054031 failed before mutation when ACTIVE_LOCAL_SOCKS_COUNT=1; later run 31911829879 passed after two immediate zero-transport proofs
rejected_hypotheses:
  - merge #988 or #1006 wholesale to preserve their useful work
  - route new client/game work to historical blakinio/otclient
  - close #1006 before execution-resource cleanup
  - treat one transient zero-socket observation as sufficient without immediate revalidation
  - weaken review or CI gates to complete the harvest
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/tasks/active/OTERYN-20260815-tibia-research-ownership-reconciliation.md
  - docs/agents/reports/OTERYN-20260815-tibia-research-ownership-reconciliation.md
validation:
  - command: cross-repository durable delivery and lifecycle verification
    result: PASS
    evidence: Oteryn-v2 #283/#284 terminal; Platform #1104 terminal
  - command: historical source PR branch closeout
    result: PASS
    evidence: #988/#1006 closed without merge with delete dispositions and exact source refs absent
  - command: #1006 execution resource cleanup
    result: PASS
    evidence: run 31911829879 / job 95077983102; TASK_RUNTIME_CLEANUP=PASS and CANONICAL_OTERYN_STAGING_UNCHANGED=true
  - command: runtime/browser E2E for archive transition
    result: NOT_APPLICABLE
    evidence: lifecycle documentation only; runtime cleanup already directly verified
blockers: []
next_action: Merge the lifecycle-only closeout PR after exact-head docs/governance validation, then verify the closeout source branch is absent.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: lifecycle-only closeout branch has no retention, rollback or recovery purpose after terminal archive delivery
source_branch_evidence: #988, #1006, #1105 and #1107 source refs are already absent; closeout branch absence must be verified immediately after merge
```

## Closeout boundary

This closeout changes lifecycle/report documentation only. It does not execute the official Tibia client, use credentials, alter canonical runtime/application code, mutate `oteryn-staging`, change production, DNS, authentication, payments, external repositories or protected environments.
