---
task_id: OTERYN-20260822-organization-runner-bootstrap-closeout
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
search_first:
  - organization runner bootstrap
  - deploy/synology/runner
optional_reads: []
---

# OTERYN-20260822-organization-runner-bootstrap-closeout

## Goal

Land the canonical organization-capable Synology runner bootstrap/hardening from stale PR #1200 onto current Platform `main` without restoring superseded activation workflows, then terminally reconcile Platform #1199 and the stale branch lifecycle.

## Acceptance criteria

- [x] Platform `main` contains the reviewed organization/repository registration entrypoint, immutable runner image inputs, self-tests and no-secret organization Compose/runbook.
- [x] Existing repository-scoped `oteryn-staging` restart compatibility remains intact.
- [x] Organization mode requires explicit group/name/custom labels, uses `--runnergroup` and `--no-default-labels`, and rejects generic default labels.
- [x] Current runner Compose retains the Platform state mount while gaining generic scope/group/token-file inputs.
- [x] Superseded activation/repair workflows from stale PR #1200 were not reintroduced.
- [x] Exact-head Platform CI/governance/security/build validation passed with no material review finding.
- [x] PR #1200 was closed as superseded; PR #1213 squash-merged on the exact validated head; Platform #1199 closed completed.
- [x] Both obsolete task source branches were verified absent after terminal disposition.

## Ownership

```yaml
owned_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
modules:
  - synology-runner-bootstrap
dependencies:
  - Oteryn/Oteryn#34
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#35
  - Oteryn/Oteryn-Game#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T02:41:30Z
head: e2032a1ec4fa04f98bc5f7d115e21a17f9e42f42
branch: infra/issue-1199-runner-bootstrap-current-main
pr: 1213
status: completed
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
proven:
  - stale PR 1200 was closed unmerged as superseded and its source branch is absent
  - PR 1213 exact head e4dc3532fd3e2d298ab0b4692a7a867cb94abf65 passed all eight applicable exact-head workflows
  - Build Synology Staging Images run 32546746461 passed package validation and deploy-runner image build/self-test; deploy-runner job 96966419001 succeeded
  - PR 1213 squash-merged as e2032a1ec4fa04f98bc5f7d115e21a17f9e42f42 and Platform #1199 closed completed
  - PR 1213 source branch is absent after merge
  - reusable runner-core changes landed without the stale activation workflows from PR 1200
  - Dockerfile pins Actions Runner 2.336.0 tarball sha256 04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d plus immutable Ubuntu and Docker CLI bases
  - organization mode requires group/name/custom label, uses --runnergroup and --no-default-labels, and self-tests fail closed on malformed/default-label inputs
derived:
  - Platform provider/bootstrap slice for parent runner programme is terminal; remaining parent work belongs to provider routing/control-plane verification outside this completed Platform task
unknown: []
conflicts: []
first_failure:
  marker: stale PR 1200 could not be safely merged onto current main
  evidence: it was 11 commits behind current main and contained superseded activation workflows; current-main PR 1213 replaced only reusable runner-core surfaces
rejected_hypotheses:
  - merge PR 1200 directly; rejected because its base diverged and it contained superseded activation workflows
changed_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
validation:
  - command: PR 1213 exact-head workflow generation at e4dc3532fd3e2d298ab0b4692a7a867cb94abf65
    result: PASS
    evidence: CI 32546746304; Agent Governance 32546746391; Synology Rollback 32546746302; Platform DB Outage 32546746423; Game Auth 32546746399; Build Synology Staging Images 32546746461; Edge Security 32546746347; Phase 7 32546746403 all succeeded
  - command: Build Synology Staging Images run 32546746461
    result: PASS
    evidence: deployment package job 96966419000 and deploy-runner build/self-test job 96966419001 succeeded; image descriptor digest sha256:3d84f36a18f616ad0fbf5ac4b88394a1b52feb132f1d99a36783bedaa029d2a1
  - command: E2E classification
    result: NOT_APPLICABLE
    evidence: repository-only bootstrap/templates perform no live registration/product mutation; executable integration contract was the build-time entrypoint self-test plus exact-head Synology package/image validation
  - command: post-merge branch verification
    result: PASS
    evidence: infra/issue-1199-runner-bootstrap-current-main and infra/issue-1199-organization-runner-bootstrap both absent
blockers:
  - none
next_action: none
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: runner-bootstrap-closeout-20260822-001
  session_started_at: 2026-08-22T02:20:00Z
  checkpointed_at: 2026-08-22T02:41:30Z
  last_progress_at: 2026-08-22T02:41:30Z
  phase: closeout
  exact_head: e2032a1ec4fa04f98bc5f7d115e21a17f9e42f42
  pull_request: 1213
  active_operation: none
  external_run_ids: [32546746302, 32546746391, 32546746423, 32546746399, 32546746461, 32546746347, 32546746403, 32546746304]
  operation_started_at: null
  wait_deadline_at: null
  check_generation: terminal
  checks_used: 2
  status: completed
  safe_to_resume: true
  resume_condition: none
  next_action: none
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: e4dc3532fd3e2d298ab0b4692a7a867cb94abf65
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - repository mode retains exact owner/repository URL validation and oteryn-synology-staging/oteryn-staging defaults
    - organization mode requires strict group/name/custom label and appends --runnergroup plus --no-default-labels
    - self-test rejects malformed URLs/groups/labels/default labels and proves restart without re-registration
    - organization reference Compose isolates registration/work volumes and withholds Docker/root from Game
    - stale PR 1200 temporary activation workflows were excluded from the current-main implementation
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository current-main successor for Platform #1199
source_branch_evidence: PR 1213 squash-merged as e2032a1ec4fa04f98bc5f7d115e21a17f9e42f42; source branch verified absent; superseded PR 1200 source branch also verified absent
```

## Notes

Platform #1199 is complete. This record does not claim that organization runner-group `Selected repositories` ACLs were directly read back; that control-plane verification remains with the provider/parent runner programme.