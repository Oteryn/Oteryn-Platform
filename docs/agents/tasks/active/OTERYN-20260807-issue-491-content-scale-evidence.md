---
task_id: OTERYN-20260807-issue-491-content-scale-evidence
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - PORTAL_CONTENT_SCALE_EVIDENCE
  - portal-exhaustive-current-main-audit
optional_reads:
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
---

# OTERYN-20260807-issue-491-content-scale-evidence

## Goal

Resolve Issue #491 fail-closed evidence-contract gaps: make content-scale coverage include every current portal surface fragment, and make final exact-head audit provenance durable and machine-verifiable without conflating generator, final PR-head, and merge identities.

## Acceptance criteria

- [x] Content-scale validation loads and classifies all 27 current portal surfaces, including fragment-defined surfaces.
- [x] Adding a fragment-defined portal surface without a content-scale classification fails deterministic validation.
- [x] Existing content-scale evidence remains fail-closed; applicable consumers require executable evidence and zero remaining gaps.
- [x] The final exact-head audit artifact identity and digest from run 30799469813 are retained in repository evidence.
- [x] Retained multi-stage audit evidence has explicit provenance and deterministic validation rejects unexplained mixed provenance.
- [x] Generator/base evidence SHA, final PR head SHA, and merge SHA remain distinct facts.
- [ ] Focused tests, exact-head self-review, applicable required CI, and closeout gates pass.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json
  - tools/audit/validate_portal_exhaustive_retained_evidence.py
  - tools/audit/test_validate_portal_exhaustive_retained_evidence.py
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/manifest.json
  - docs/agents/tasks/active/OTERYN-20260807-issue-491-content-scale-evidence.md
modules:
  - testing
  - quality_e2e
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T17:11:39Z
head: bf51724f94c5f43fa3205930f60669775d321765
branch: fix/issue-491-content-scale-evidence
pr: 821
status: validating
phase: terminal_ci
execution_mode: github
execution_reason: repository-only repair is fully implemented; GitHub Actions and protected PR merge are the remaining proving path
invocation_started_at: 2026-08-07T17:06:00Z
last_progress_at: 2026-08-07T17:11:39Z
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: 2026-08-07T17:09:59Z
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
context_routes:
  - testing
  - agent-governance
owned_paths:
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json
  - tools/audit/validate_portal_exhaustive_retained_evidence.py
  - tools/audit/test_validate_portal_exhaustive_retained_evidence.py
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/manifest.json
  - docs/agents/tasks/active/OTERYN-20260807-issue-491-content-scale-evidence.md
proven:
  - Issue #491 remains open and is the repair target of existing PR #821; no duplicate repair PR was created.
  - Content-scale validation now loads the base portal coverage manifest plus surface fragments and classifies all 27 current portal surfaces.
  - Nine fragment-defined surfaces have explicit applicability rationales and a deterministic negative fixture rejects a newly introduced unclassified fragment surface.
  - Applicable content-scale consumers remain fail-closed: executable evidence mappings are required and the complete contract permits zero gap surfaces.
  - Final PR-head audit run 30799469813 tested e4c16048288ba9a9bd699a7c3427495105922503 and uploaded artifact 8850222872 with digest sha256:548fc6a906b5c482b535a8bcc158604f9b89788e87b1d326deb7ca5fe58b55b5; that identity is retained in the repository manifest.
  - Retained audit provenance explicitly distinguishes base generator f5f83b8122fa266bb8f7dc45019fea566ac53fb5, strict source 67ed852cdd973c9265401190561d968226348649, final tested PR head e4c16048288ba9a9bd699a7c3427495105922503, and merge cbbd7613cee13cf01931a0ba0f7ac089122132e0.
  - Deterministic retained-evidence validation rejects unexplained embedded source SHAs and identity conflation.
  - Full PR diff on bf51724f94c5f43fa3205930f60669775d321765 remains within the eight declared owned paths and contains no product runtime, production, secret, schema, payment, data or cross-repository mutation.
  - Exact-head self-review on bf51724f94c5f43fa3205930f60669775d321765 passed with zero material findings; negative fixtures cover unclassified fragment surfaces, unexplained or malformed embedded SHAs, digest validity, durable final-reference mismatch, SHA identity conflation and strict-source mismatch.
  - First aggregate terminal-CI observation on bf51724f94c5f43fa3205930f60669775d321765 had nine successful workflow families and two still in progress: Content Scale Acceptance 31200474910 and Deep System Validation 31200471446.
derived:
  - The remaining gate is final exact-head CI plus protected merge and lifecycle closeout; the task is eligible for bounded terminal-CI continuation.
unknown:
  - Final conclusions of Content Scale Acceptance 31200474910 and Deep System Validation 31200471446 on bf51724f94c5f43fa3205930f60669775d321765.
conflicts: []
first_failure:
  marker: branch_pr_identity_omitted
  evidence: Agent Governance on head 9dab42759ee11809a9b9a97c2bba4529a8185bc7 reported that fix/issue-491-content-scale-evidence already has open PR #821 and the task must record it; this bookkeeping defect is repaired on later heads.
rejected_hypotheses:
  - Issue #801 as takeover target: its branch and PR #825 moved during recovery, proving another agent retained live ownership.
  - Product/runtime regression on reviewed #491 head: the repair diff is repository CI/evidence-contract only and all completed exact-head workflow families are green.
changed_paths:
  - .github/workflows/portal-exhaustive-audit.yml
  - docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/manifest.json
  - docs/agents/tasks/active/OTERYN-20260807-issue-491-content-scale-evidence.md
  - docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-content-scale-evidence.mjs
  - tools/audit/test_validate_portal_exhaustive_retained_evidence.py
  - tools/audit/validate_portal_exhaustive_retained_evidence.py
validation:
  - command: exact-head PR workflows on 9dab42759ee11809a9b9a97c2bba4529a8185bc7
    result: FAIL
    evidence: ten applicable workflow families passed; Agent Governance failed only with branch_pr_identity_omitted for PR #821.
  - command: exact-head self-review of PR #821 changed paths on bf51724f94c5f43fa3205930f60669775d321765
    result: PASS
    evidence: full eight-path diff and acceptance were checked; fail-closed validators and deterministic negative fixtures are present; zero material findings.
  - command: first aggregate terminal-CI observation on bf51724f94c5f43fa3205930f60669775d321765
    result: NOT_RUN
    evidence: nine workflow families passed; Content Scale Acceptance 31200474910 and Deep System Validation 31200471446 remained in progress.
blockers:
  - none
next_action: Re-read the live PR head and aggregate required checks after the minimum terminal-CI interval; if every exact-head gate passes, squash-merge PR #821 and complete Issue/task archival closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chatgpt-20260807-1906
  session_started_at: 2026-08-07T17:06:00Z
  checkpointed_at: 2026-08-07T17:11:39Z
  last_progress_at: 2026-08-07T17:11:39Z
  phase: terminal_ci
  exact_head: bf51724f94c5f43fa3205930f60669775d321765
  pull_request: 821
  active_operation: final required exact-head CI and protected merge readiness
  external_run_ids:
    - 31200474910
    - 31200474944
    - 31200471446
    - 31200471390
    - 31200471415
    - 31200471441
    - 31200471311
    - 31200471884
    - 31200471900
    - 31200471411
    - 31200472043
  operation_started_at: 2026-08-07T17:09:59Z
  wait_deadline_at: 2026-08-07T17:54:59Z
  check_generation: ready
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: every required workflow on the live final PR head is terminal and successful
  next_action: Re-read the live PR head and aggregate required checks after the minimum terminal-CI interval; if every exact-head gate passes, squash-merge PR #821 and complete Issue/task archival closeout.
```

## Notes

Risk gate: HEIGHTENED because this repair changes CI/evidence-contract enforcement, while remaining repository-only with no production, secrets, live data, schema, payment, or cross-repository mutation.
