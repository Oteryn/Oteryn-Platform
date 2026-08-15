# Tibia research ownership reconciliation — terminal state

Coordination ID: `OTER-CLIENT-REFERENCE-HARVEST-20260815`.

## FACT — ownership is reconciled

- Native Oteryn client/game/protocol/world semantics belong to `blakinio/Oteryn-v2` under the accepted Oteryn-Game lineage.
- `blakinio/otclient` is historical migration/reference evidence and is not a destination for new Oteryn v2 work.
- Platform retains only reusable host/control-plane reference infrastructure that is genuinely a Platform concern.

## FACT — durable delivery is complete

Oteryn-v2 PR #283 merged the proprietary-data-free worldmap/client-world reference package as `0c307db73832b824ccf50801e626671e0aeb38d1`; lifecycle closeout PR #284 merged as `5d40711074dd914e0fcf8a95954180d84feef5f3`.

Platform PR #1104 hardened and merged the reusable #988 official Linux reference infrastructure as `c014fcad498e2568cc47b64f6d886967f270d7a1`. Its final head `f4ecbe1bfdd7a51940901d1dd236cdb968da1d44` passed CI `31911457035`, Phase 7 `31911457015`, Platform DB Outage `31911457054`, Agent Governance `31911457030` and Tibia Linux Reference Harness `31911457036` after all material review findings were repaired.

Historical PR #988 was closed without merge with `Branch-Disposition: delete`; its source branch is absent. Issue #987 deliberately remains open/blocked because dedicated-host official client acquisition/execution/BattlEye acceptance is not completed.

## FACT — #1006 execution resources and branch are terminal

A fresh read-only session rerun job `95077830819` proved the client PID existed but had zero local SOCKS and zero direct TCP.

The immediately-following ownership-scoped cleanup run `31911829879`, job `95077983102`, revalidated the exact runner, labels, task identity and bind and again proved zero transport before mutation. It removed only the task-owned relay if present, container `oteryn-tibia-client-analysis`, and `/volume1/docker/oteryn/tibia-analysis`.

The bind contained `1171069521` bytes before removal. Post-cleanup checks proved the task resources were absent and the canonical `oteryn-staging` inventory was unchanged:

```text
TASK_RUNTIME_CLEANUP=PASS
CANONICAL_OTERYN_STAGING_UNCHANGED=true
```

PR #1006 was then closed without merge with `Branch-Disposition: delete`; its exact source branch is absent.

The first failed-close cleanup attempt #1105 and the successful one-shot cleanup workflow #1107 were both closed without merge with delete dispositions; their temporary branches are absent.

## FACT — excluded research scaffolding was not promoted

No branch-only live-client login/recovery workflow, VNC, private-message action, gdb/ptrace/live-attach experiment, blind movement workflow, screenshot/base64 evidence, credential/session material or proprietary Tibia binary/asset from #1006 was promoted into Platform `main` or Oteryn-v2.

## Remaining work outside this reconciliation

Issue #987 remains a separate blocked official-offline-validation research gate. Other future research questions such as exact player XYZ, passability/collision classification, raw Worldmap semantic mapping, higher-level native action ABI and complete OTBM-recoverable coverage are not claimed complete by this ownership/branch cleanup.

## Status

`COMPLETED` — repository ownership, durable harvest, historical PR disposition, source-ref cleanup and task-owned runtime cleanup are all proven. The only remaining action is lifecycle-only archival of this reconciliation task itself.
