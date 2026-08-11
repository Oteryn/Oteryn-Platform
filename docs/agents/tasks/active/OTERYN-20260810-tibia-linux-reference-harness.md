---
task_id: OTERYN-20260810-tibia-linux-reference-harness
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
search_first:
  - OTERYN-20260810-tibia-linux-reference-harness
  - official Linux client
  - tibia-linux-reference
historical_reference:
  - PR #391
---

# OTERYN-20260810 Tibia Linux reference harness

## Policy

```yaml
policy_version: 2
task_kind: e2e
implementation_authorized: true
merge_authorized: true
external_service_validation_authorized: false
external_service_execution_ready: false
context_pressure: medium
context_growth: stable
decomposition_decision: phased
execution_mode: github_only
phase: local-synthetic-readiness
```

## Goal

Restore only the still-useful synthetic/no-network Linux reference harness from historical PR #391 onto current `main`, fix the historical graphical CI failure without weakening security controls, and leave a current, inspectable foundation for a separately owner-gated official-component phase.

This task does **not** authorize an official Tibia login, official-service contact, real credentials, an anti-cheat workaround, deployment, or implementation changes in Oteryn-v2/OTClient/Canary.

## Current authority

- `blakinio/Oteryn-Platform`: Identity/Gateway/control-plane interoperability workflow.
- `blakinio/Oteryn-v2`: canonical native Rust client/game/protocol implementation authority; read-only here.
- historical OTClient, Canary and official Tibia behavior: reference/compatibility evidence only.
- PR #391: historical source for reusable synthetic harness code only; it remains closed and must not be merged or reopened for this continuation.
- Owner continuation instruction on 2026-08-11 authorizes normal repository closeout, including merge when the repository merge gate is satisfied. It does not authorize deployment or any external-service interaction.

## Security invariants

- Automated validation uses generated synthetic secrets only.
- No real credential, account password, authenticator code, token or session material enters chat, Git, GitHub, arguments, ordinary environment variables, retained logs, screenshots or artifacts.
- No official binary or proprietary asset is committed.
- No official service is contacted in this phase.
- No `LD_PRELOAD`, ptrace, debugger attachment, hooking, injection, memory access, binary patching, traffic decryption/replay/injection or anti-cheat bypass.
- Raw runtime evidence, if a later phase is authorized, remains on an encrypted private volume outside Git.
- Security claims are bounded to the documented threat model and enumerated tested surfaces; this task does not claim that leakage is impossible under arbitrary host compromise.

## Execution modes

- `fake-offline`: current phase; deterministic fake graphical client, IPv4/IPv6/DNS denied by OS isolation, synthetic secrets only.
- `official-offline-launch`: future component gate only; exact approved package identity, dedicated interactive Linux host, encrypted private evidence volume, no authentication.
- `official-live`: unavailable in this task; requires separate explicit owner authorization and local secret entry.

## Scope

### Authorized

- carry forward the historical synthetic harness after current-main revalidation;
- fix the synthetic no-network CI failure at its bounded root cause;
- add fail-closed preflight, identity, redaction, leak-scan, cleanup, manifest and X11 ABI tests;
- keep CI synthetic-only and free of official packages and repository/organization secrets in the harness process;
- update the task, plan, observation and PR with redacted evidence;
- complete normal PR review, exact-head validation, squash merge and task archival when all repository gates pass.

### Not authorized

- official-service authentication or world entry;
- official client/BattlEye execution in this task;
- official client/BattlEye modification or bypass;
- gameplay automation;
- network interception/decryption/replay/injection;
- cross-repository writes;
- deployment or production mutation.

## Owned paths

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260810-tibia-linux-reference-harness.md
  - docs/agents/reports/OTERYN-20260810-tibia-linux-reference-harness-plan.md
  - docs/agents/reports/OTERYN-20260810-tibia-linux-reference-harness-observation.md
  - tools/tibia-linux-reference/**
  - .github/workflows/tibia-linux-live-reference.yml
read_only_cross_repository:
  - blakinio/Oteryn-v2
  - blakinio/otclient
  - project-owned Canary repository
```

## Acceptance criteria

- [x] Historical harness is deliberately carried forward onto a fresh branch from the then-current `main`; later `main` changes were checked for overlap.
- [x] CI is synthetic-only and the harness process does not inherit GitHub runtime credentials or persisted checkout credentials.
- [x] Fake graphical client starts in an isolated X11 session using filesystem-local display transport.
- [x] Dry-run network denial proves only loopback is visible and bounded IPv4, IPv6 and `.invalid` DNS probes are denied.
- [x] Generated synthetic secrets are transported without arguments or ordinary environment variables.
- [x] Exact synthetic-secret and high-confidence leak scans pass on the enumerated Git/process/output/evidence/temp/history surfaces.
- [x] Cleanup verification passes, including task-owned Xvfb diagnostics.
- [x] Identity/package verification and encrypted-evidence preflight remain fail-closed foundations for a future separately authorized official mode.
- [x] Manifest schema and synthetic example validate under the fail-closed safety contract.
- [x] No official login, official-client execution, BattlEye execution or official-service contact occurs.
- [x] Focused validation and the dedicated GitHub harness workflow pass on runtime implementation head `cd4ab6c68e2cd20e19f80ddbb9a4c14535223f3d` and final pre-wait head `ffa5b6910f28fd1ad96e2116c690d61caba5af76`.
- [x] Exact-head full-diff self-review is PASS with no open material findings, requested changes or review threads on PR #961.
- [ ] Final required repository governance/broad checks pass after the unrelated terminal Wiki task is archived from current `main`.
- [ ] PR #961 becomes terminal and the task is archived after merge.
- [x] `external_service_execution_ready` remains `false` throughout this task.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-linux-reference-owner
  classified_at: 2026-08-11T09:53:00+02:00
  risk: high
  triggers:
    - synthetic credential and secret-handling controls
    - OS network isolation and negative-path proof
    - CI credential isolation
    - future fail-closed official-component identity and encrypted-storage foundation
  unknown_or_conflict: []
  rationale: The delivered phase is synthetic-only, but it touches security-sensitive secret, process, network, CI and future identity boundaries, so heightened negative-path and exact-head evidence is required.
  self_review:
    result: PASS
    exact_head: ffa5b6910f28fd1ad96e2116c690d61caba5af76
    acceptance_checked: true
    full_diff_checked: true
    negative_paths_checked: true
    rollback_checked: true
    compatibility_checked: true
    related_prs_checked: true
    findings: []
    evidence:
      - PR #961 contains 23 task-owned changed paths and no changes in Oteryn-v2, OTClient, Canary, deployment or production surfaces.
      - Tibia Linux Reference Harness run 31470266075 job 93711827807 is SUCCESS on head ffa5b6910f28fd1ad96e2116c690d61caba5af76.
      - Runtime gate run 31469809872 job 93710431162 proves 29 tests, manifest validation, graphical lifecycle, IPv4/IPv6/DNS denial, leak scan and cleanup PASS.
      - PR #961 has zero submitted reviews requesting changes and zero review threads.
      - Historical PR #391 remains intentionally closed/superseded; Wiki lifecycle PR #981 is a separate non-overlapping repository-governance dependency.
      - Rollback is bounded to reverting/removing additive research tooling, its isolated workflow, and documentation; no production schema/data/deployment mutation exists.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T09:53:00+02:00
head: ffa5b6910f28fd1ad96e2116c690d61caba5af76
branch: feat/OTERYN-20260810-tibia-linux-reference-harness
pr: 961
status: waiting
terminal_pr_policy: archive_pending
invocation_started_at: 2026-08-11T09:00:00+02:00
last_progress_at: 2026-08-11T09:53:00+02:00
ci_checks_for_current_head: 3
ci_check_generation: ready-current-base
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 2
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
context_reconstruction_attempts: 1
stall_warnings: 1
context_routes:
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260810-tibia-linux-reference-harness.md
  - docs/agents/reports/OTERYN-20260810-tibia-linux-reference-harness-plan.md
  - docs/agents/reports/OTERYN-20260810-tibia-linux-reference-harness-observation.md
  - tools/tibia-linux-reference/**
  - .github/workflows/tibia-linux-live-reference.yml
proven:
  - Historical PR 391 is closed and superseded; PR 961 is the active continuation for this scope and is ready-for-review.
  - Current main is 5a687af557da7368ae7f1872d698a6246fce8853, the merge commit of terminal Wiki implementation PR #972.
  - Tibia Linux Reference Harness run 31470266075 job 93711827807 on head ffa5b6910f28fd1ad96e2116c690d61caba5af76 passed the synthetic graphical no-network gate and every step.
  - Runtime implementation run 31469809872 job 93710431162 passed compileall, 29 focused tests, manifest validation, graphical no-network execution, leak scan, cleanup and no-raw-artifact verification.
  - The synthetic manifest evidence reports only interface lo in a distinct namespace, network denial proven, official endpoint contact false, raw capture false, leak scan PASS and cleanup PASS.
  - Game Auth Ticket Concurrency run 31470266047, Phase 7 Production-Like Validation run 31470266039 and Edge Security Emulation run 31470266033 are SUCCESS on PR head ffa5b6910f28fd1ad96e2116c690d61caba5af76.
  - CI runtime-tests job 93712115118 is SUCCESS; a later repository-selected CI job remains queued.
  - Agent Governance run 31470266028 validated this task checkpoint and live ownership before failing global liveness because the already merged Wiki task from PR #972 remained in docs/agents/tasks/active on current main.
  - Separate closeout PR #981 exists for that Wiki task; its Agent Governance run 31470406536 is SUCCESS, but PR #981 currently has one unresolved material Codex review thread requiring exact-head self-review evidence and its CI is still queued.
  - PR #961 has no review threads and no requested-change review.
  - Exact-head self-review for ffa5b6910f28fd1ad96e2116c690d61caba5af76 is PASS.
derived:
  - The current merge blocker is repository-global lifecycle drift owned by PR #981, not a defect in the Tibia Linux reference harness or its task checkpoint.
  - Mutating the Wiki closeout record from PR #961 would violate the current task owned-path boundary and one-owner remediation model.
  - The synthetic harness is ready for terminal closeout once the external governance dependency is terminal and required current-base checks are regenerated.
unknown: []
conflicts: []
first_failure:
  marker: resolved-x11-ctypes-abi
  workflow_run: 31469315399
  job: 93708936795
  evidence: the previous fake-client-unknown graphical failure was resolved by pointer-safe X11 ABI declarations; runs 31469809872 and 31470266075 are green
rejected_hypotheses:
  - Focused syntax, unit, manifest, or storage-encryption tests are the graphical failure source; all 29 focused tests pass in the successful runtime run.
  - Missing X11 filesystem socket is the graphical failure source; the workflow proves /tmp/.X11-unix/X99 exists before launch.
  - Current-main overlap is the harness failure source; current main changed the unrelated Wiki task and the PR merge ref remained mergeable.
  - GitHub credential persistence remains available to harness Git subprocesses; checkout removes temporary authentication before harness execution and persist-credentials is false.
  - Agent Governance failure on final PR head proves this task checkpoint is invalid; its own checkpoint and ownership steps pass, while failure evidence names only stale terminal Wiki task OTERYN-20260810-wiki-expected-content-inventory / PR #972.
changed_paths:
  - .github/workflows/tibia-linux-live-reference.yml
  - docs/agents/tasks/active/OTERYN-20260810-tibia-linux-reference-harness.md
  - docs/agents/reports/OTERYN-20260810-tibia-linux-reference-harness-plan.md
  - docs/agents/reports/OTERYN-20260810-tibia-linux-reference-harness-observation.md
  - tools/tibia-linux-reference/**
validation:
  - command: Tibia Linux Reference Harness run 31470266075 job 93711827807
    result: PASS
    evidence: all steps success on head ffa5b6910f28fd1ad96e2116c690d61caba5af76
  - command: Tibia Linux Reference Harness run 31469809872 job 93710431162
    result: PASS
    evidence: 29 tests PASS; manifest PASS; graphical component PASS; leak scan PASS; cleanup PASS; only lo visible; official endpoint contact false; raw capture false
  - command: Agent Governance run 31470266028 job 93711827517
    result: FAIL
    evidence: this task checkpoint validation and live ownership passed; global liveness failed only on stale merged Wiki task PR #972
  - command: PR #981 Agent Governance run 31470406536
    result: PASS
    evidence: the separate Wiki archival PR fixes the global stale-task category, but remains non-terminal with one unresolved review finding
blockers:
  - Repository-global Agent Governance cannot pass for PR #961 while merged PR #972 remains represented as an active Wiki task on current main; the authorized owner has opened PR #981 to archive it, but #981 is not yet terminal.
next_action: After PR #981 becomes terminal on main, refresh PR #961 against that current base and run the required exact-head merge gates.
```
