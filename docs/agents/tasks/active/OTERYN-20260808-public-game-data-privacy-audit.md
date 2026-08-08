---
task_id: OTERYN-20260808-public-game-data-privacy-audit
repository: blakinio/Oteryn-Platform
programme: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: Audit
execution_mode: github
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
---

# OTERYN-20260808 PublicGameData privacy audit

## Goal

Audit the native PublicGameData projection/reconciliation contract delivered by Issue #902 / PR #903 for security and semantic completeness around Platform-owned privacy, stale serving, caches/search/CDN, projection generations and rollback. Route material gaps to an independent remediation Issue without implementing them in the audit role.

## Audited scope

Protected `main@bb51c0329b8907502ea1162ff632df7ba968855d` and:

- `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`;
- `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`;
- Issue #902 / PR #903 delivery semantics;
- current `PublicCharacterProfileService` direct privacy composition;
- current `CharacterProfilePreferenceService` and preference event recording, only as compatibility/reference evidence;
- baseline Issues #486/#487 for duplicate ownership checking.

Issue #905 exclusively owns the stale continuous-audit programme-state correction. This audit does not touch that programme file or Issue #908 remediation paths.

## Acceptance criteria

- [x] Protected main and live ownership were refreshed after the previous audit closeout.
- [x] PublicGameData source freshness, last-known-good, cache/CDN and privacy overlay semantics were reconciled.
- [x] Current Canary-compatible direct-read behavior was distinguished from future native projection/cache architecture.
- [x] Baseline/privacy/PublicGameData Issues were checked for duplicate ownership.
- [x] One material security-contract gap was routed to Issue #908 (`OPA-SEC-0004`).
- [x] No contract repair, runtime/schema/cache/CDN/deployment/credential/production/external-repository mutation occurred.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` for this architecture audit package.
- [ ] Exact-head Agent Governance and repository-selected CI pass; complete diff and review-thread state have zero unresolved material findings.
- [ ] Delivery PR is merged and lifecycle closeout archives this task.

## Finding

```yaml
finding_id: OPA-SEC-0004
issue: 908
severity: high
priority: P1
confidence: high
evidence_state: PROVEN
disposition: open_ready_remediation
summary: The native PublicGameData contract permits last-known-good stale public serving and requires privacy invalidation, but does not define privacy decision revision/freshness, restrictive-change cutoff, failed/ambiguous invalidation behavior, privacy dependency outage behavior, or rollback rules that prevent an older cached allow from surviving a newer deny.
```

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-audit.md
  - docs/agents/tasks/archive/OTERYN-20260808-public-game-data-privacy-audit.md
  - docs/agents/reports/OTERYN-20260808-public-game-data-privacy-audit.md
modules:
  - public-game-data
  - privacy
  - security
  - continuous-audit
forbidden_paths:
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - app/**
  - services/**
  - database/**
  - deploy/**
  - .github/workflows/**
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T13:46:00+02:00
head: pending
branch: audit/OTERYN-20260808-public-game-data-privacy
pr: none
status: validating
context_routes:
  - agent-governance
  - architecture
  - security
  - product
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-audit.md
  - docs/agents/tasks/archive/OTERYN-20260808-public-game-data-privacy-audit.md
  - docs/agents/reports/OTERYN-20260808-public-game-data-privacy-audit.md
proven:
  - Protected main at audit start is bb51c0329b8907502ea1162ff632df7ba968855d.
  - PR #903 defines HTTP/API/SSR/cache/CDN consumption of a Platform normalized public read model and permits last-known-good stale serving within family freshness policy when game-source evidence is temporarily stale.
  - PR #903 correctly makes CharacterProfiles/Identity privacy an independent upper bound and states game-originated public=true cannot override it.
  - The contract states privacy changes must invalidate/rebuild affected public presentation/cache/search state.
  - The contract validation requires privacy denies to remain effective over fresh game facts and after cache/search refresh.
  - The contract does not specify a privacy decision revision/generation/watermark or equivalent ordering proof for composed public variants.
  - It does not specify the authoritative cutoff for a newer restrictive privacy change against already-cached/search/CDN output.
  - It does not specify failed/delayed/ambiguous privacy invalidation or privacy-policy dependency-unavailable semantics distinct from game-source unavailability.
  - It does not specify how rollback to an older game-projection generation is prevented from resurrecting a public variant preceding a newer privacy deny.
  - Current Canary-compatible PublicCharacterProfileService reads CharacterProfilePreference during request composition rather than proving the future native cache ambiguity is a current runtime defect.
  - Current CharacterProfilePreferenceService writes preferences transactionally and records preference-updated security events; no native PublicGameData cache implementation exists in this audit scope.
  - Baseline Issues #486/#487 own broad capability/evidence gaps, not this native privacy-revocation security contract.
  - Issue #908 now durably owns OPA-SEC-0004 as P1/high agent-ready remediation.
derived:
  - An implementation could satisfy game-source stale-while-servable rules while continuing to expose an older allowed public variant after a newer privacy deny if purge/rebuild is delayed or fails.
  - Game projection freshness and privacy decision freshness require separate security semantics.
unknown:
  - Exact future native cache/CDN/search technology and invalidation mechanism remain intentionally undecided.
  - Exact privacy revision representation is a remediation design choice; the audit requires monotonic/equivalently safe semantics, not a specific storage mechanism.
conflicts:
  - OPA-SEC-0004 / Issue #908 owns PublicGameData privacy-revocation contract reconciliation and must not be implemented by this audit.
  - OPA-GOV-0031 / Issue #905 independently owns the continuous-audit programme file and is outside this task.
first_failure:
  marker: privacy-deny-has-no-versioned-public-cutoff
  evidence: Contract allows last-known-good public variants and mandates privacy invalidation, but has no ordering/failure rule proving an older allow cannot remain serveable after a newer deny.
rejected_hypotheses:
  - Current PublicCharacterProfileService is already leaking hidden fields; rejected because it directly reads preference state during request composition and no concrete leak was proven.
  - Game-source hard-expiry rules automatically protect privacy; rejected because game fact freshness and Platform privacy authorization are independent authorities.
  - Generic baseline #486/#487 already owns this exact gap; rejected because those findings cover broad capability/evidence completeness, not native projection privacy revocation semantics.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-audit.md
  - docs/agents/reports/OTERYN-20260808-public-game-data-privacy-audit.md
validation:
  - command: live main/ownership preflight
    result: PASS
    evidence: Main, Issue #905, active tasks and recent audit closeout were refreshed before selecting #903 as the next non-overlapping domain.
  - command: PublicGameData privacy/freshness semantic reconciliation
    result: PASS
    evidence: Last-known-good, hard expiry, privacy overlay, cache/CDN and validation clauses were compared as one security boundary.
  - command: current compatibility implementation cross-check
    result: PASS
    evidence: PublicCharacterProfileService and CharacterProfilePreferenceService were inspected to avoid falsely declaring the future native ambiguity a current runtime leak.
  - command: duplicate finding search
    result: PASS
    evidence: No open Issue owned this native privacy-revocation contract; #486/#487 are broader baseline owners.
  - command: runtime browser E2E
    result: NOT_APPLICABLE
    evidence: Audit/architecture documentation only.
  - command: exact-head Agent Governance and repository-selected CI
    result: NOT_RUN
    evidence: Run after bounded audit PR creation and PR identity binding.
blockers: []
next_action: Open the bounded audit PR, record its PR identity, run exact-head governance/CI, inspect the full diff and review threads, merge only if every gate passes, then archive the audit task without implementing Issue #908.
invocation_started_at: 2026-08-08T13:43:00+02:00
last_progress_at: 2026-08-08T13:46:00+02:00
ci_checks_for_current_head: 0
ci_check_generation: public-game-data-privacy-audit
terminal_ci_wait_started_at: none
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
```
