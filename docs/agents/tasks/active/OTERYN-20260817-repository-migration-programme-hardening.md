---
task_id: OTERYN-20260817-repository-migration-programme-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/tasks/archive/OTERYN-20260817-repository-migration-ultra-overlay.md
search_first:
  - PR #1135 canonical programme plus thin Ultra overlay model
  - current active task ownership and open PR changed paths
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION hardening or equivalent active branch
optional_reads: []
---

# OTERYN-20260817-repository-migration-programme-hardening

## Goal

Harden the canonical `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION` programme while preserving PR #1135's canonical-programme plus thin-Ultra-overlay composition. Limit the change to the canonical programme, the Ultra evaluation, the minimum Ultra delta and this controlled task record.

Trusted-base reconstruction: `blakinio/Oteryn-Platform@fcafc20bc9705ca92256fdddc7433bcc3d191c40` (`main`, observed 2026-08-17).

Branch: `docs/oteryn-20260817-repository-migration-programme-hardening`.
PR: `#1138`.

## Acceptance criteria

- [x] Current root and `docs/agents` governance was read from the trusted base.
- [x] Live `main`, active tasks, open PRs and branch ownership were reconstructed before editing.
- [x] No active task or open PR owns the four changed paths at preflight.
- [x] The canonical programme remains the sole migration authority.
- [x] The Ultra file remains a thin execution overlay rather than a replacement mega-prompt.
- [x] Hardening covers authority narrowing, evidence leases, target collision, bounded risk acceptance, single-mutation transactions, rollback-versus-redirect, drift invalidation and docs-only completion semantics.
- [x] The Ultra evaluation covers positive, negative, boundary, drift, collision, rollback, unavailable-tooling and unreproduced-review cases.
- [x] The controlled branch exists from the verified `main` base.
- [x] Exactly one PR is open for this task: #1138.
- [x] Current PR diff is restricted to exactly the four owned paths.
- [ ] Full exact-head self-review has zero unresolved material findings.
- [ ] Repository-required Agent Governance and CI checks pass on the unchanged final PR head.
- [ ] Review-thread hygiene and source-branch lifecycle gates are terminal.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-programme-hardening.md
modules:
  - agent-governance
  - prompt-as-code
  - repository-migration-programme
dependencies:
  - PR #1135 merged canonical-programme plus thin-Ultra-overlay model
  - trusted main fcafc20bc9705ca92256fdddc7433bcc3d191c40
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 3
updated_at: 2026-08-17
base_head: fcafc20bc9705ca92256fdddc7433bcc3d191c40
branch: docs/oteryn-20260817-repository-migration-programme-hardening
pr: 1138
status: validating
context_routes:
  - agent-governance
  - prompt-as-code
  - ecosystem-repository-migration
proven:
  - Live main was reconstructed at fcafc20bc9705ca92256fdddc7433bcc3d191c40 before the controlled branch was created.
  - PR #1135 established the canonical-programme plus thin-Ultra-overlay model.
  - The active tasks and open PRs inspected at preflight did not own any of the four candidate paths.
  - The remote task branch was created from the verified main head.
  - Exactly one draft PR exists for this task: #1138.
  - GitHub compare and PR metadata confirm exactly four changed files, matching the owned-path allowlist.
  - Full-diff audit found one material wording defect in the canonical programme (`stronger_only`); it was repaired on the same branch before final exact-head gates.
  - No physical repository coordinate, production runtime, deployment, credential, secret or live-game mutation is part of this hardening.
derived:
  - This increment is documentation and prompt-governance only, so executable migration E2E is not applicable.
unknown:
  - The exact external critical-review V2 artifact is unavailable in this execution context; no unreproduced review claim is promoted into canonical authority.
  - Final exact-head CI, review-thread and merge/readiness state remain unresolved until repository-required gates run on the frozen PR head.
conflicts: []
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-programme-hardening.md
validation:
  - check: changed-path allowlist
    result: PASS
    evidence: GitHub compare and PR metadata report exactly the four owned paths.
  - check: full-diff audit repair
    result: PASS
    evidence: material `stronger_only` wording defect was replaced by the exact reviewed canonical candidate wording before final gate execution.
  - check: E2E
    result: NOT_APPLICABLE
    evidence: documentation and prompt-governance hardening only; no executable repository/control-plane effect changed.
  - check: exact-head Agent Governance and CI
    result: PENDING
    evidence: final frozen PR head must be selected by repository-required checks.
next_action: Freeze the PR head, inspect the full exact PR diff again, then observe repository-required Agent Governance and CI without changing the head unless a material finding requires repair.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: branch is active for PR #1138 and must remain until PR lifecycle is terminal
source_branch_evidence: docs/oteryn-20260817-repository-migration-programme-hardening
```

## Notes

The candidate intentionally does not edit `docs/agents/SHORT_PROGRAM_INVOCATIONS.md`, programme runtime state, architecture ADRs or any non-Platform repository. The exact external V2 review text was unavailable, so only findings independently reproduced against current canonical files were incorporated.
