---
task_id: OTERYN-20260807-branch-lifecycle-remote-identity-postrepair-audit
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
status: validating
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: audit/branch-lifecycle-remote-identity-postrepair-20260807
base_branch: main
base_sha: 5041a669a811f47fe11b3e6dec0993a28cfa26d7
pr: none
production_activation_authorized: false
cross_repository_mutation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
search_first:
  - Issue #815 and repair PR #822
  - active tasks and open PRs overlapping Branch Lifecycle
optional_reads: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity-postrepair-audit.md
modules:
  - branch-lifecycle-remote-identity
coordination_key: audit:branch-lifecycle-remote-identity-postrepair
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 Branch Lifecycle remote-identity post-repair audit

## Goal

Independently re-audit OPA-GOV-0024 / Issue #815 after repair PR #822, proving that destructive Branch Lifecycle deletion is bound to the configured repository root and the same GitHub owner/name identity used for live safety reads, while retaining exact force-with-lease atomicity.

## Scope and collision check

Read-only audited product/tooling paths:

- `tools/agents/branch_lifecycle.py`
- `tools/agents/test_branch_lifecycle.py`
- `.github/workflows/branch-lifecycle.yml`
- `docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md`
- `docs/agents/BRANCH_LIFECYCLE_POLICY.json`

Issue #815 is closed completed. Repair PR #822 merged as `da0ae1e792a90bb0774b6028195b9f4519f50516` from exact implementation head `911837bed2daa57be59323395bf0552d67de05a1`. No open Branch Lifecycle PR or active task currently owns the audited paths. Comparing repair merge through selected `main@5041a669a811f47fe11b3e6dec0993a28cfa26d7` shows no later change to the audited Branch Lifecycle implementation, tests or workflow.

No destructive branch operation, workflow change, product/runtime change or external-repository mutation is authorized by this audit.

## Audit result

**PASS — no new material defect is proven in the bounded post-repair destructive remote-identity boundary.**

## Current safety chain

`GitHubClient` now receives an explicit repository root and stores its resolved path as `git_root`. Every git subprocess used by the destructive boundary runs with `cwd=self.git_root`.

Before any deletion push, `_validated_git_remote()`:

1. executes `git rev-parse --show-toplevel` under the configured root;
2. requires the reported worktree root to resolve exactly to `git_root`;
3. resolves all push URLs for the configured git remote;
4. requires exactly one push URL;
5. accepts only supported GitHub HTTPS/SSH forms;
6. normalizes the URL to `owner/name`;
7. requires that normalized identity to equal `GitHubClient.repo` case-insensitively.

Only after those checks does `_delete_ref_with_lease()` execute:

`git push --porcelain --force-with-lease=refs/heads/<branch>:<expected_sha> <validated_remote> :refs/heads/<branch>`

The expected SHA must be a full 40-character object ID. A rejected push re-reads the authoritative GitHub ref and distinguishes lease drift, ambiguous missing-ref result and unchanged-ref rejection without treating the failure as success.

## Negative and boundary evidence

The repair regression suite directly covers:

- supported GitHub HTTPS, SCP-like SSH and `ssh://git@github.com/...` normalization;
- rejection of foreign hosts, file remotes, extra path components and credential-bearing HTTPS URLs;
- foreign GitHub repository identity rejected before destructive push;
- configured root differing from the actual git worktree rejected before remote lookup/push;
- ambiguous multiple push URLs rejected before destructive push;
- all git subprocesses executing with the configured root as `cwd`;
- exact expected-SHA force-with-lease deletion semantics;
- the last-instruction race where the remote ref advances after review and the lease rejects deletion.

The original Issue #815 mismatch — GitHub API safety reads describing repository A while local `origin`/CWD could delete repository B — is therefore not present in the inspected repaired path.

## Workflow integration

`.github/workflows/branch-lifecycle.yml` runs focused Branch Lifecycle tests and a read-only live inventory on applicable PRs. The protected-main apply lane is separately gated by reviewed candidate approval, rebuilds live evidence, verifies the protected default branch, checks candidate/policy hashes and then invokes the same Branch Lifecycle apply path. This audit does not execute that destructive apply lane.

## Validation evidence

PR #822 exact implementation head `911837bed2daa57be59323395bf0552d67de05a1`:

- Branch Lifecycle `31186492049`: **PASS**.
- CI `31186491768`: **PASS**.
- Agent Governance `31186491920`: **PASS**.
- Platform DB Outage Validation `31186491834`: **PASS**.
- Game Auth Ticket Concurrency `31186491846`: **PASS**.
- Edge Security Emulation `31186492703`: **PASS**.
- Phase 7 Production-Like Validation `31186492156`: **PASS**.

## Findings and deduplication

No new material finding is proven and no new Issue is created.

- OPA-GOV-0024 / Issue #815 remains the historical identity of the pre-repair remote/CWD authority defect.
- Issue #793 remains the historical expected-SHA atomicity predecessor; PR #822 preserves its force-with-lease semantics rather than replacing them.
- No open Branch Lifecycle audit/repair owner overlaps this post-repair verification.

## Acceptance inventory

- [x] Live main, Issue #815, repair PR #822, active tasks and open PR ownership were refreshed.
- [x] Repair-to-current-main delta shows no audited-path change after PR #822.
- [x] Configured-root binding and remote owner/name normalization were inspected in current source.
- [x] Missing/foreign/ambiguous/wrong-root negative paths and supported URL forms have deterministic regression coverage.
- [x] Exact expected-SHA force-with-lease atomicity remains intact.
- [x] Exact repair-head Branch Lifecycle, CI and Agent Governance evidence passed.
- [x] No destructive validation and no runtime/tooling fix were performed by this audit.
- [ ] Exact-head CI / Agent Governance for this audit record pass and PR hygiene is clean.
- [ ] Lifecycle closeout archives the task and advances programme state.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-07T20:18:00+02:00
invocation_started_at: 2026-08-07T19:51:00+02:00
last_progress_at: 2026-08-07T20:18:00+02:00
head: 5041a669a811f47fe11b3e6dec0993a28cfa26d7
branch: audit/branch-lifecycle-remote-identity-postrepair-20260807
pr: none
status: validating
phase: final_ci
execution_mode: github_only
context_routes:
  - continuous-audit
  - ci-build-test
  - architecture-governance
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity-postrepair-audit.md
proven:
  - Issue #815 is closed completed through PR #822.
  - Branch Lifecycle implementation/test/workflow paths are unchanged after repair merge through selected main.
  - Destructive git operations are rooted at configured git_root and validate the exact worktree root before push.
  - The selected push remote must normalize to GitHubClient.repo and exactly one supported push URL.
  - Exact expected-SHA force-with-lease deletion and last-instruction race rejection remain covered.
derived:
  - The OPA-GOV-0024 cross-repository destructive-authority defect is repaired in the inspected bounded path.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A wrong CWD can still select the destructive git repository; rejected because every git subprocess is bound to git_root and root identity is checked.
  - A foreign GitHub origin with the same branch/SHA can still receive deletion; rejected because normalized remote repository identity must match GitHubClient.repo before push.
  - The remote-identity repair weakened Issue #793 atomicity; rejected because the exact expected-SHA force-with-lease remains in the destructive command and race regression.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity-postrepair-audit.md
validation:
  - command: PR #822 Branch Lifecycle 31186492049
    result: PASS
    evidence: exact implementation-head Branch Lifecycle workflow passed.
  - command: PR #822 CI 31186491768
    result: PASS
    evidence: exact implementation-head repository CI passed.
  - command: PR #822 Agent Governance 31186491920
    result: PASS
    evidence: exact implementation-head governance passed.
  - command: audit-document E2E
    result: NOT_APPLICABLE
    evidence: audit changes evidence only; destructive live execution is intentionally not used as audit proof.
blockers:
  - none
ci_checks_for_current_head: 0
ci_check_generation: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: open the bounded audit-record PR, bind its numeric identity, require exact-head CI and Agent Governance, then merge and archive if hygiene stays clean
```

## Safety

Repository-only read-mostly audit. No branch deletion, production operation, secret access, external repository mutation or workflow/runtime change was performed.
