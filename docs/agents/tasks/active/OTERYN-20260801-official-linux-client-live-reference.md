---
task_id: OTERYN-20260801-official-linux-client-live-reference
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
search_first:
  - OTERYN-20260801-official-linux-client-live-reference
  - official Linux client
  - Oteryn-v2
optional_reads:
  - docs/agents/tasks/archive/OTERYN-20260727-tibia-linux-runner-analysis.md
---

# OTERYN-20260801 official Linux client live reference

## Current authority status

`BLOCKED RESEARCH CAPABILITY / HISTORICAL PR — OTERYN-V2 TARGET AUTHORITY RECONCILED`

This task began before the accepted native Oteryn-v2 repository/authority transition. Its safe synthetic Linux harness remains useful research tooling, but its old `blakinio/otclient` / Canary implementation handoff is no longer current architecture.

Current authority from protected `main` is recorded by ADR 0031 and Issue #886:

- `blakinio/Oteryn-Platform` owns Identity, OAuth/PKCE, Game Login Ticket, Gateway pre-admission/routing and Platform-side interoperability workflow;
- `blakinio/Oteryn-v2` is the canonical native client/game/protocol implementation authority and remains read-only from this task;
- historical `blakinio/otclient`, Canary and official Tibia behavior are compatibility/reference evidence only;
- live official-client observation can inform requirements but never proves native Oteryn-v2 conformance by itself.

Because this is an intentionally historical unmerged branch, current-main ADR files are referenced through Issue #886 rather than made local required reads. A future continuation must start from current `main` and load the then-current architecture authority before reuse.

## Goal

Preserve a safe, reproducible research capability for launching the unmodified official Tibia Linux client and, only after all owner-gated prerequisites are genuinely satisfied, collecting a bounded redacted interoperability observation that can be compared with the current Oteryn architecture.

Any final gap matrix must route findings by current authority:

- Platform Identity/Gateway/control-plane findings -> `blakinio/Oteryn-Platform`;
- native Rust client, native game server and `protocol-oteryn` findings -> separately authorized `blakinio/Oteryn-v2` follow-up;
- OTClient/Canary/Tibia findings -> historical/compatibility/reference evidence unless a later explicit compatibility task adopts them.

No cross-repository source mutation belongs to this task.

## Safety invariants

Live execution is allowed only when all of these are true:

- the account is owned and explicitly designated by the repository owner;
- credentials are supplied only through a local manual/ephemeral mechanism and never through ChatGPT, GitHub, shell arguments, ordinary environment variables, logs or artifacts;
- the official client and BattlEye remain unmodified;
- there is no patching, hooking, injection, process-memory modification, anti-cheat bypass, traffic alteration, replay, cheating or autonomous gameplay;
- raw sensitive evidence remains only on a private encrypted research volume outside Git;
- GitHub receives only redacted text evidence, hashes and non-sensitive manifests;
- the bounded observation stops on anti-cheat/account-security warnings, credential exposure risk or scope expansion.

## Non-goals

This task does not authorize:

- implementation changes in Oteryn-v2, historical otclient or Canary;
- gameplay automation, farming, combat, trade or interaction with other players;
- interception/decryption/modification of official traffic;
- redistribution of official binaries/assets;
- production changes;
- treating a reachable endpoint, successful login or one observed session as proof of complete protocol compatibility;
- using historical OTClient/Canary correspondence as final native Oteryn-v2 conformance.

## Preserved harness evidence

The unique synthetic/no-network work in PR #391 remains valuable and must not be discarded merely because the target architecture changed.

Proven on the historical branch:

- compact Python harness under `tools/tibia-linux-reference/`;
- deterministic fake graphical client;
- separate loopback-only network namespace with denied reserved-address connection;
- exact synthetic-secret scanning and deterministic cleanup;
- redacted session-manifest schema/example;
- official-mode preflight that fails closed when encrypted storage or exact package identity is unavailable;
- focused tests, Python compilation and manifest/workflow/checkpoint validation;
- no official client, official service, credential or proprietary asset was used during the proven synthetic run.

Historical exact local/code evidence includes `cabad487a139aaf0983dfc55cfb18d9f43720633`; PR #391 later advanced to `630ed73c09242cf3d37f3652b06fa252c6b0f10d`. These hashes preserve evidence only; they are not current-main conformance claims.

## External-service readiness

`external_service_execution_ready: false`

The live phase remains blocked because the task has not proven all required prerequisites together:

1. a dedicated interactive Linux research host or other environment accepted by the unmodified official client/BattlEye;
2. a provably encrypted private evidence volume outside Git;
3. the exact owner-approved official package path and expected package/binary identity;
4. successful no-authentication official component launch without bypass/modification;
5. owner-gated exact account/character and secure manual/ephemeral credential mechanism;
6. the exact minimal observation script for the requested evidence gap.

No worker may weaken these gates merely to make progress.

## Phases

### 1. Local synthetic harness — PROVEN

Keep the fake-client, no-network, redaction and cleanup capability as reusable reference tooling.

### 2. Official component gate — BLOCKED

Only verify the exact unmodified official package identity and launch it without authentication on an approved encrypted research host. A BattlEye/environment refusal is terminal evidence for that environment and must not be worked around.

### 3. Owner-gated live observation — BLOCKED

Only after Phase 2 and explicit owner-supplied local prerequisites are satisfied may one bounded official login/world-entry observation be performed. Credentials never enter repository/chat state.

### 4. Analyze and hand off — CURRENT AUTHORITY

Classify every observation as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT` and route it by current authority:

| Observation domain | Current follow-up authority | Reference-only sources |
|---|---|---|
| Identity/login/ticket/Gateway routing | Oteryn Platform | historical login/Canary paths |
| Native Rust client behavior | Oteryn-v2 | historical `blakinio/otclient`, official client behavior |
| Native gameplay protocol semantics | Oteryn-v2 | Canary/Tibia protocol observations |
| Native game/world state semantics | Oteryn-v2 | Canary runtime observations |
| Legacy compatibility behavior | explicit future compatibility task | OTClient/Canary/Tibia evidence |

No external repository task is created or mutated by this Platform task. A later follow-up requires separate authorization in its owning repository.

## Owned paths

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md
  - tools/tibia-linux-reference/**
  - .github/workflows/tibia-linux-live-reference.yml
private_runtime_paths:
  - encrypted research volume outside Git checkout
read_only_cross_repository:
  - blakinio/Oteryn-v2
  - blakinio/otclient # historical/reference only
  - project-owned Canary repository # compatibility/reference only
```

Issue #886 may edit only the three documentation paths. It must not change the validated harness or workflow.

## Acceptance criteria for any future live-reference continuation

- [x] Synthetic no-network harness and redaction/cleanup controls are preserved.
- [x] Current native implementation authority is Oteryn-v2 rather than historical `blakinio/otclient`.
- [x] Historical OTClient/Canary/Tibia evidence is explicitly reference/compatibility-only.
- [x] `external_service_execution_ready` remains false while required prerequisites are missing.
- [ ] Dedicated encrypted research environment and exact official package identity are proven.
- [ ] Unmodified official client/BattlEye component launch succeeds without authentication, or the first decisive refusal is recorded without bypass.
- [ ] Owner separately provides the final local account/character/secret-entry prerequisites and approves the exact minimal live observation.
- [ ] Any live run, if ever performed, produces only redacted GitHub-safe evidence and proves cleanup.
- [ ] Final gap matrix routes native implementation findings to Oteryn-v2 authority and Platform findings to Oteryn-Platform.
- [ ] Any implementation follow-up is separately authorized in its owning repository.

## Lifecycle disposition of PR #391

PR #391 must not be merged from its historical base merely to preserve the harness. The safe harness remains recoverable from the PR/branch and Git history.

After Issue #886 reconciles this task/plan/observation and PR metadata, PR #391 should be closed as a superseded historical research branch. If the owner later chooses to resume live-reference research, create a fresh current-main bounded task/PR and deliberately carry forward only the still-useful harness pieces after revalidation.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
phase: authority-reconciliation
session_id: github-20260808-issue886
session_role: architecture-repair
execution_mode: github_only
updated_at: 2026-08-08T10:15:00+02:00
head: pending-validation-commit
branch: repair/issue-886
pr: 896
status: blocked
context_routes:
  - agent-governance
  - architecture
  - security
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md
proven:
  - The synthetic no-network harness and cleanup/redaction evidence exist on historical PR 391 and remain recoverable without merging the stale branch.
  - Oteryn-v2 is now the canonical native client/game/protocol implementation authority; historical blakinio/otclient and Canary are reference/compatibility evidence only.
  - No official service authentication or official client execution occurred in the proven harness phase.
  - Encrypted evidence storage and exact approved official package identity remain unproven/unavailable for a live phase.
  - The historical PR 391 base head 630ed73c09242cf3d37f3652b06fa252c6b0f10d already has a failing Tibia Linux Reference Harness workflow; Issue 886 does not mutate that harness or workflow.
derived:
  - PR 391 should be superseded after documentation reconciliation rather than merged from its historical base.
  - The Tibia Linux Reference Harness failure on the stacked documentation PR is inherited from the historical base and is additional evidence against merging PR 391, not a reason to weaken the scanner in Issue 886.
unknown:
  - Whether a future approved Linux host will run the unmodified official client/BattlEye.
  - Whether the owner will choose to resume the live-reference programme after the native-v2 architecture transition.
conflicts: []
first_failure:
  marker: external-service-readiness
  evidence: encrypted evidence storage and exact approved package identity are not proven together; external_service_execution_ready remains false
rejected_hypotheses:
  - blakinio/otclient is the current native Rust-client implementation authority
  - historical Canary/OTClient correspondence proves native Oteryn-v2 conformance
  - preserving the harness requires blindly merging PR 391
  - Issue 886 should weaken or modify the credential-sensitive harness to make a historical workflow green
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md
validation:
  - command: authority reconciliation against protected-main ADR 0031 / Issue 886 evidence
    result: PASS
    evidence: native follow-up ownership now routes to Oteryn-v2; Platform responsibilities remain separate
  - command: harness/workflow mutation check
    result: PASS
    evidence: Issue 886 changes no tools/tibia-linux-reference or workflow file
  - command: official-client or official-service validation
    result: BLOCKED
    evidence: encrypted storage and exact approved package identity are not proven; no official service is contacted by this repair
  - command: historical Tibia Linux Reference Harness workflow
    result: BLOCKED
    evidence: the base head 630ed73c09242cf3d37f3652b06fa252c6b0f10d already fails this workflow; Issue 886 preserves the harness unchanged and will supersede PR 391 instead of weakening it
  - command: exact-head Agent Governance
    result: NOT_RUN
    evidence: rerun is required on the checkpoint repair head
blockers:
  - encrypted private evidence storage not proven
  - exact approved official client package identity unavailable
  - no current owner-gated live-session request for a specific native-v2 evidence gap
next_action: Revalidate the three-file documentation repair; if Agent Governance passes and the only remaining workflow failure is the proven historical harness baseline, merge the stacked reconciliation into the historical branch, update PR 391 metadata, and close PR 391 as superseded without merging it to main.
```
