---
task_id: OTERYN-20260809-download-artifact-immutability-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-content
task_kind: audit
implementation_authorized: false
execution_mode: github
execution_reason: Download Center security audit is fully evidenced in the canonical WWW Platform repository
status: validating
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
search_first:
  - Issue #948
  - PR #161
  - historical Download Center closeout Issues #562/#622/#647/#656/#676/#679/#682
  - active audit-repair findings, active tasks and open PR ownership
optional_reads: []
---

# OTERYN-20260809-download-artifact-immutability-audit

## Goal

Audit the delivered Download Center artifact-reference boundary against the accepted requirement that public client releases reference immutable, operator-approved artifacts rather than merely well-formed URLs on a trusted host.

## Acceptance criteria

- [x] Refresh protected main, active tasks, open PRs and independent audit-repair owners.
- [x] Preserve non-overlapping independent owners.
- [x] Audit the accepted Download Center architecture and delivered PR #161 implementation.
- [x] Falsify artifact-reference immutability with an approved-host mutable-reference negative path.
- [x] Distinguish the finding from truthful administrator-supplied checksum/no-fetch semantics.
- [x] Deduplicate and route OPA-SEC-0008 / Issue #948.
- [x] Keep Issue #948 remediation paths forbidden to the auditor.
- [x] Open audit PR #949 and preserve unrelated #947/#950 lifecycle work.
- [ ] Complete exact-final-head self-review, fresh review, required CI, zero unresolved material threads, merge and lifecycle closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability-audit.md
  - docs/agents/reports/OTERYN-20260809-download-artifact-immutability-audit.md
modules:
  - downloads
  - security
  - architecture-governance
dependencies:
  - Issue #948 independent remediation owner
  - PR #161 delivered Download Center
blockers:
  - none
cross_repository_tasks:
  - none
forbidden_paths:
  - app/Downloads/**
  - app/Http/Requests/Downloads/**
  - app/Http/Controllers/Downloads/**
  - config/downloads.php
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - database/**
  - routes/**
  - resources/**
  - .github/workflows/**
  - deploy/**
  - external repositories
```

## Audit result

One material finding is proven:

- **OPA-SEC-0008 / Issue #948 — HIGH / P1**: accepted Download Center architecture requires immutable artifact references, but `ArtifactUrlPolicy` accepts any HTTPS URL on an exact allowlisted host when it has any non-root path. An approved mutable alias/object key therefore passes the control without object-version, content-address binding or equivalent immutability proof.

The public checksum notice remains truthful: SHA-256 is administrator-supplied and Platform explicitly does not fetch or independently verify the artifact. That deliberate boundary is not the finding.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T10:02:00Z
invocation_started_at: 2026-08-09T09:36:00Z
last_progress_at: 2026-08-09T10:02:00Z
head: OUT_OF_BAND_FINAL_HEAD_AFTER_THIS_CHECKPOINT
branch: audit/OTERYN-20260809-download-artifact-immutability
pr: 949
status: validating
phase: exact-head-validation
session_id: agent-20260809-0936-download-artifact-immutability
session_role: auditor
project_lane: oteryn-platform-content
execution_mode: github
context_routes:
  - agent-governance
  - security
  - public-web-cms
  - architecture
  - downloads
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability-audit.md
  - docs/agents/reports/OTERYN-20260809-download-artifact-immutability-audit.md
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
validation_level: documentation_exact_head
invocation_budget_minutes: 60
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
proven:
  - Protected main at audit selection was c1b1d26b355db26a89d983cc4abc6477bf843a26.
  - Unrelated federated-search repair PR #947 advanced main to a82ec651f9155fc5acbfe78d6c3b792fa9b9c0b8 and was incorporated without semantic audit changes.
  - The first final-head Agent Governance attempt failed only because #947 had merged while its separate active repair task still awaited lifecycle closeout; this was external to #949 and did not invalidate the audit checkpoint schema or finding.
  - Independent lifecycle PR #950 subsequently archived the #938 repair task and merged as b87deb370c4a0a629a8aaf05d0447134f2ee823e, resolving that repository-wide liveness condition without this auditor touching the other owner's paths.
  - Current protected main incorporated for the next validation generation is b87deb370c4a0a629a8aaf05d0447134f2ee823e through the PR merge base; #949 still changes only its two audit docs.
  - Active durable tasks at selection were public-domain repair and native-auth production-verification plus `.gitkeep`.
  - Issues #941 and #944 remain independent non-overlapping finding owners at selection; Issue #938 was repaired/closed separately.
  - PR #338 is an intentional Game Catalog compatibility hold and PR #541 is public-domain external evidence work.
  - PUBLIC_WEBSITE_EXPANSION_PLAN requires immutable artifact URL or approved storage reference and immutable operator-approved artifacts.
  - ArtifactUrlPolicy enforces HTTPS, exact configured host, no userinfo, no fragment, standard HTTPS port and a non-root path, but no content-address, object-version, digest binding or host-specific immutable-reference proof.
  - PublishClientRelease re-runs the same insufficient policy immediately before publication.
  - SaveClientReleaseRequest requires SHA-256 syntax but does not bind that supplied digest to the artifact reference.
  - Public Download Center truthfully states SHA-256 is administrator-supplied and not independently verified by Platform.
  - Existing focused tests do not reject mutable aliases/overwriteable paths on an otherwise approved host.
  - Duplicate searches found no Issue owning this exact artifact-reference immutability root cause.
  - OPA-SEC-0008 / Issue #948 independently owns remediation on deterministic branch repair/issue-948.
  - Audit PR #949 contains the task plus matching report and no remediation/product paths.
derived:
  - Database-row immutability does not make externally addressed bytes immutable when the approved URL can resolve to replaced content.
  - Manual checksum comparison is useful but is not equivalent to the architecture's machine-enforced immutable-reference invariant.
unknown: []
conflicts: []
first_failure:
  marker: repository-wide-terminal-task-liveness
  evidence: Agent Governance run 31307054812 failed because merged PR #947 remained represented by its separately owned active task before closeout #950; #950 later merged and removed that condition.
rejected_hypotheses:
  - Exact host allowlisting proves artifact immutability; it constrains origin but not overwriteability.
  - A non-root or version-looking pathname proves immutable bytes; naming convention is not a storage immutability proof.
  - Lack of independent checksum verification is the defect; that limitation is explicitly disclosed and outside this root cause.
  - Historical Download Center lifecycle Issues already own the defect; they repaired ownership lifecycle and preserved supplied-checksum/no-fetch semantics.
  - The Agent Governance failure was caused by #949; logs identify the stale terminal #947 active task instead.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability-audit.md
  - docs/agents/reports/OTERYN-20260809-download-artifact-immutability-audit.md
validation:
  - command: live main / ownership reconciliation
    result: PASS
    evidence: non-overlapping audit selected from c1b1d26b355db26a89d983cc4abc6477bf843a26; #947 and its lifecycle closeout #950 remain separately owned and incorporated through current main
  - command: architecture / PR #161 negative-path review
    result: PASS
    evidence: approved-host mutable reference satisfies current policy despite immutable-reference architecture requirement
  - command: duplicate search
    result: PASS
    evidence: no exact duplicate; Issue #948 created as independent remediation owner
  - command: CI run 31307054820 on prior head 4ca8acbcfff3d892649b182098635301ccf42c7b
    result: PASS
    evidence: classify-changes PASS; test PASS; runtime-tests SKIPPED
  - command: fresh Codex review on prior head 4ca8acbcfff3d892649b182098635301ccf42c7b
    result: PASS
    evidence: no major issues
  - command: Agent Governance run 31307054812 on prior head 4ca8acbcfff3d892649b182098635301ccf42c7b
    result: FAIL
    evidence: repository-wide #947 terminal-task lifecycle remained active until independently resolved by merged closeout #950
  - command: runtime/browser E2E for audit deliverable
    result: NOT_APPLICABLE
    evidence: audit documentation only; no product behavior changes
  - command: exact-final-head self-review / fresh review / repository CI after this reconciliation
    result: NOT_RUN
    evidence: this checkpoint reconciliation creates the final validation generation after #950 resolved the external liveness gate
blockers: []
next_action: Validate the new exact PR #949 head against current main with self-review, fresh Codex review, Agent Governance/CI, exact changed paths and zero unresolved threads; merge only if all gates pass, then perform separate lifecycle archive/programme reconciliation.
```
