---
task_id: OTERYN-20260727-tibia-linux-runner-analysis
required_reads:
  - AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/reports/OTERYN-20260727-research-purpose-and-safety-scope.md
search_first:
  - OTERYN-20260727-tibia-linux-runner-analysis
  - PR 218
optional_reads: []
---

# OTERYN-20260727-tibia-linux-runner-analysis

## Goal

Produce a bounded, text-only interoperability audit of the official Tibia Linux client: reconstruct protected/unprotected game-server routing, current login transport and BattlEye integration boundaries; compare them with OTClient/Oteryn/Canary; and preserve a responsible-research boundary without modifying staging services, implementing a bypass or redistributing proprietary material.

## Policy

```yaml
policy_version: 2
task_kind: audit
implementation_authorized: false
external_service_validation_authorized: false
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive audit completed through bounded static-analysis, callback, reporting and closeout phases
execution_mode: chat
execution_reason: remaining work is narrow GitHub state reconciliation, documentation and exact-head validation coordination
```

## Completion decision

The repository-side static audit is complete. Official-server acceptance and BattlEye enforcement remain `UNKNOWN` / `NOT_RUN` and are explicitly outside the current authorization. They do not block closure of this audit and must be handled only by a separate task with explicit owner authorization and risk acceptance.

The current PR is not merge-ready only because its branch is materially stale against `main` and exact-final-head checks have not yet been re-established after restack.

## Acceptance criteria

- [x] Analysis and report-recovery work used runner label `oteryn-staging` without staging secrets or services.
- [x] Downloaded client bytes remained only in Docker volume `oteryn-tibia-linux-analysis` on Synology.
- [x] Exact analyzed ELF version, size, SHA-256 and Build ID are recorded.
- [x] Loginservice world fields are mapped to protected and unprotected endpoints.
- [x] `TGameserverDualConnection` scheduling, primary/secondary behavior, fallback and backoff are reconstructed.
- [x] The final `QTcpSocket::connectToHost` path is identified.
- [x] The challenge-driven protobuf login structure is reconstructed to the supported evidence level; unresolved names remain unknown.
- [x] The semantic roles and immediate application effects of all three callbacks supplied to `BEClient.so::Init` are recorded.
- [x] Current OTClient compatibility is assessed against the analyzed 15.30 protocol.
- [x] Confirmed behavior, derived conclusions, unknown server controls and rejected hypotheses are separated.
- [x] The safe validation boundary and responsible-disclosure threshold are documented.
- [x] Final report, callback addendum and Policy v2 safety scope are committed.
- [x] Temporary automatic analysis workflows are removed.
- [x] No CipSoft binary, package, asset, credential, cookie or session secret is committed or uploaded as a GitHub artifact.
- [ ] PR #218 is restacked onto current `main`, exact-final-head required checks pass, and the task is archived after merge.

## Ownership

```yaml
owned_paths:
  - .github/workflows/tibia-linux-runner-analysis.yml
  - .github/workflows/tibia-linux-report-artifact.yml
  - docs/agents/tasks/active/OTERYN-20260727-tibia-linux-runner-analysis.md
  - docs/agents/reports/OTERYN-20260727-research-purpose-and-safety-scope.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-protected-route-analysis.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-battleye-callback-addendum.md
modules:
  - agent governance and task lifecycle
  - GitHub Actions static-analysis infrastructure
  - static protocol and transport analysis
dependencies:
  - current main branch for final restack
  - exact-final-head repository checks
blockers: []
cross_repository_tasks:
  - blakinio/otclient was inspected read-only for compatibility evidence
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
phase: close
session_id: chat-20260801-policy-v2-closeout
session_role: coordinator
execution_mode: chat
execution_reason: narrow documentation, live PR/CI reconciliation and closeout preparation
updated_at: 2026-08-01T09:30:00Z
lease_expires_at: 2026-08-01T10:30:00Z
head: f87fc64504171275709f0ab8035847f45c3eecd7
branch: ci/OTERYN-20260727-tibia-linux-runner-analysis
pr: 218
status: validating
context_routes:
  - governance
  - testing
  - security
  - cross-repository
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: the audit is cohesive; only restack, exact-head validation, merge and archive remain
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 1
stale_takeover_count: 0
human_interruptions: 0
last_completed_step: aligned the durable safety and completion boundary with Prompting Policy v2
owned_paths:
  - .github/workflows/tibia-linux-runner-analysis.yml
  - .github/workflows/tibia-linux-report-artifact.yml
  - docs/agents/tasks/active/OTERYN-20260727-tibia-linux-runner-analysis.md
  - docs/agents/reports/OTERYN-20260727-research-purpose-and-safety-scope.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-protected-route-analysis.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-battleye-callback-addendum.md
proven:
  - PR 218 is open, draft and mergeable with head branch ci/OTERYN-20260727-tibia-linux-runner-analysis.
  - Before the Policy v2 closeout edits, PR head ef4f504575cef807722e9dd7c9b8731fa7a0be62 was 136 commits behind current main and the branches had diverged.
  - The persistent analysis run is 30246435256-1 and the analyzed client SHA-256 is 8b25d65ece158723dbb50a1b592c1ec8a3247a650fcd2d299bebdfd133cb5752.
  - externaladdressprotected and externalportprotected form the protected route; externaladdressunprotected and externalportunprotected form the unprotected route.
  - Protected is scheduled at time T and unprotected at T plus 1 millisecond.
  - With optimizeConnectionStability disabled only one connection is attempted or active at a time; when enabled both routes may be in flight.
  - The first connected route is primary and a later connected route is secondary; route identity is not permanently synonymous with primary or secondary.
  - EConnectionsUsed values are none 0, unprotected 1, protected 2 and both 3.
  - Route failure backoff is 1, 2, 4, 8 and then 16 seconds.
  - The selected QUrl host and port reach QTcpSocket connectToHost.
  - The official 15.30 login path is challenge-driven protobuf, not the current OTClient legacy RSA/XTEA login message.
  - BEClient.so is loaded through QLibrary, Init is resolved, the protected endpoint is supplied and three callbacks are registered.
  - Callback 0x7418f0 is a diagnostic log sink and has no direct network effect.
  - Callback 0x741de0 records or emits restart-required state and has no direct network effect.
  - Callback 0x741b50 copies an opaque BattlEye-produced byte buffer and emits an application Qt signal; no direct socket write was found.
  - All temporary analysis and report-recovery workflows were removed from the PR branch.
  - The final report, callback addendum and Policy v2 safety scope exist on the task branch.
  - Current-head CI, Agent Governance, Platform DB Outage Validation, Game Auth Ticket Concurrency and Phase 7 Production-Like Validation passed on ef4f504575cef807722e9dd7c9b8731fa7a0be62.
  - Edge Security Emulation failed on that stale head because tests/edge-emulation/bin/curl did not exist; the failure is unrelated to the four documentation files in PR 218 and is consistent with branch staleness.
derived:
  - Protected is normally primary and unprotected normally fallback or secondary, but connection order determines primary/secondary state.
  - TCP reachability of the unprotected endpoint does not demonstrate game-session acceptance or a security defect.
  - Current OTClient requires substantial protobuf, framing, challenge and secondary-connection work before it can interoperate with this client generation.
  - Static client analysis cannot prove server-required fields, official-server acceptance or protected-world enforcement.
  - The repository-side audit can close without an official-service test because that test is outside current authorization and its absence is explicitly recorded as UNKNOWN/NOT_RUN.
  - Restacking is required before interpreting repository-wide CI or merging PR 218.
unknown:
  - The exact downstream Tibia envelope and message type used for bytes emitted by callback 0x741b50.
  - Whether BEClient.so contributes bytes to the first game-server authentication request.
  - Exact generated protobuf names for all nested login fields and the numeric outer message type for GameclientMessageLogin.
  - Which fields are mandatory and independently validated by the official server.
  - Whether an official protected or optional world accepts a session without valid BattlEye state or data.
conflicts: []
first_failure:
  marker: exact-final-head validation unavailable until PR 218 is restacked
  evidence: branch comparison showed ci/OTERYN-20260727-tibia-linux-runner-analysis 136 commits behind current main; stale-head Edge Security Emulation failed at chmod tests/edge-emulation/bin/curl because the path was absent
rejected_hypotheses:
  - Address 0xc49630 is the protected/unprotected selector.
  - The branch on world-record offset 0x69 is a BattlEye route selector.
  - Functions near 0x722200 and 0x7226e0 are route selectors.
  - Arbitrary dlopen/dlsym references prove BEClient.so loading.
  - The official 15.30 first login packet is the historical OTClient RSA/XTEA block.
  - Primary and secondary are permanently synonymous with protected and unprotected.
  - Callback 0x7418f0 sends network data.
  - Callback 0x741de0 sends network data.
  - An official-service live test is required to complete the current static audit.
changed_paths:
  - docs/agents/reports/OTERYN-20260727-research-purpose-and-safety-scope.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-protected-route-analysis.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-battleye-callback-addendum.md
  - docs/agents/tasks/active/OTERYN-20260727-tibia-linux-runner-analysis.md
validation:
  - command: workflow run 30493514149
    result: PASS
    evidence: persistent volume inventory verified the exact client hash and text-report corpus
  - command: workflow run 30493947069
    result: PASS
    evidence: focused text-only report recovery uploaded one-day artifact 8740881984
  - command: workflow run 30495579436 rerun job 90815528899
    result: PASS
    evidence: bounded callback and protobuf extraction uploaded text-only artifact 8752558221
  - command: workflow run 30526055084 job 90817099201
    result: PASS
    evidence: exact callback literals and bounded context uploaded text-only artifact 8752725268
  - command: localhost synthetic login-and-one-turn dry-run
    result: PASS
    evidence: challenge, login, login success, one turn, state update and disconnect completed against a local mock only
  - command: minimal own-account official-server acceptance test
    result: NOT_RUN
    evidence: outside current authorization; retained as UNKNOWN and not a completion blocker
  - command: temporary workflow cleanup
    result: PASS
    evidence: temporary callback extraction and report-recovery workflows were deleted after use
  - command: Prompting Policy v2 scope review
    result: PASS
    evidence: safety scope now declares audit authorization, evidence model, separate live-validation gate, completion boundary and stop conditions
  - command: exact-final-head repository checks after restack
    result: NOT_RUN
    evidence: branch remains stale and must be restacked before final validation
blockers: []
next_action: Restack PR 218 onto current main without broadening its four-file scope, run exact-final-head required checks, then merge and archive the task if the diff and checks are clean.
```

## Durable reports

- `docs/agents/reports/OTERYN-20260727-research-purpose-and-safety-scope.md`
- `docs/agents/reports/OTERYN-20260727-tibia-linux-protected-route-analysis.md`
- `docs/agents/reports/OTERYN-20260727-tibia-linux-battleye-callback-addendum.md`

## Final response contract

```text
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <compact result>
VALIDATION: <checks and outcomes>
DURABLE_STATE: <task path, branch, head, PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```
