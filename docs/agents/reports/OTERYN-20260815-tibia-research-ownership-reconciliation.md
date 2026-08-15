# Tibia research ownership reconciliation — current durable state

Coordination ID: `OTER-CLIENT-REFERENCE-HARVEST-20260815`.

## Verified ownership

- Native Oteryn client/game/protocol/world semantics belong to the Oteryn-Game lineage, currently `blakinio/Oteryn-v2`.
- `blakinio/otclient` is historical migration/reference evidence and receives no new Oteryn v2 implementation work.
- Platform owns reusable infrastructure/reference execution tooling when it is a Platform host/control-plane concern rather than native game code.

## Completed delivery

### Platform PR #988 harvest

Historical source: `blakinio/Oteryn-Platform#988`, head `f9ff34b37cf81c400a48f7ab9329393416ac304d`.

Reusable official archive identity, dedicated-host/real-UID/direct-rendering preflight, Ubuntu dedicated-user preparation, interactive LUKS2 evidence-volume setup and focused tests were reworked on current Platform lineage. Review repairs made the destructive and future execution paths fail closed:

- `wipefs` inspection failure stops before formatting;
- the documented `official-component` CLI enforces dedicated-host/real-UID/accelerated-graphics gates before launcher delegation;
- dedicated-user identity is derived from `os.getuid()` plus the password database rather than environment variables.

Clean Platform PR #1104 final head `f4ecbe1bfdd7a51940901d1dd236cdb968da1d44` passed CI `31911457035`, Phase 7 `31911457015`, Platform DB Outage `31911457054`, Agent Governance `31911457030` and Tibia Linux Reference Harness `31911457036`, then merged as `c014fcad498e2568cc47b64f6d886967f270d7a1`.

PR #988 was subsequently closed without merge with `Branch-Disposition: delete`; its source branch is absent. Issue #987 remains open/blocked because dedicated-host official package acquisition/execution/BattlEye acceptance is still unperformed.

### Platform PR #1006 durable client/world artifact

Historical source: `blakinio/Oteryn-Platform#1006`, head `97f8df9e64e1e4f0520440073e497f24dad929ef`.

The proprietary-data-free `tools/tibia-worldmap-reconstruction/**` package and durable client/world ownership evidence were migrated to Oteryn-v2 rather than merged with the 302-commit/76-file Platform research history.

Oteryn-v2 PR #283 repaired four automated-review findings, passed exact-head merge/governance/reference validation, and merged as `0c307db73832b824ccf50801e626671e0aeb38d1`. Lifecycle closeout PR #284 merged as `5d40711074dd914e0fcf8a95954180d84feef5f3`; the active task is absent and the terminal archive is present on Oteryn-v2 `main`.

The migration deliberately excludes Platform live-client/login/VNC/private-message/gdb/ptrace/blind-movement workflows, screenshots/base64 evidence, credentials/session material and proprietary binaries.

## Inherited Platform repair

Fresh Platform CI exposed an existing ADR registry defect unrelated to the Tibia harvest: ADR 0040 declared supersession without the required explicit `- Superseded by:` target. PR #1104 added the missing target to ADR 0041. The resulting exact-head CI generation passed.

## Remaining blocker — PR #1006 runtime closeout

PR #1006 is intentionally still open and must not be wholesale merged or deleted yet.

Read-only session-check rerun job `95075794423` observed the client PID with zero local/direct TCP at one instant. The immediately following ownership-scoped terminal-cleanup attempt run `31911054031` / job `95076020397` revalidated the exact runner/container labels/mount and then observed:

```text
CLIENT_PID_PRESENT=true
ACTIVE_LOCAL_SOCKS_COUNT=1
ACTIVE_DIRECT_TCP_COUNT=0
```

The precondition failed. Every destructive cleanup step was skipped. The attempt did not intentionally mutate canonical `oteryn-staging` or remove the task container/bind.

Ephemeral cleanup-attempt PR #1105 was closed without merge with `Branch-Disposition: delete`; its source branch is absent.

PR #1006 comment `5304478532` records the canonical migration destinations and runtime blocker. Its branch remains intentionally live until cleanup can be proven safe.

## Exact next action

Run the existing read-only #1006 session check again. Only when the check proves no active client transport, immediately perform ownership-scoped removal of exactly:

- container `oteryn-tibia-client-analysis`;
- bind `/volume1/docker/oteryn/tibia-analysis`;
- any independently verified task-owned `oteryn-tibia-vnc-relay`.

The cleanup must compare canonical `oteryn-staging` inventory before/after and fail closed on any ownership or session ambiguity. After cleanup PASS, close #1006 without merge using `Branch-Disposition: delete` and verify source-ref removal.

## Status

`WAITING` — repository ownership and durable harvest are complete; only #1006 task-owned live-runtime closeout remains. No polling or destructive action is justified while the local SOCKS transport survives/reconnects.
