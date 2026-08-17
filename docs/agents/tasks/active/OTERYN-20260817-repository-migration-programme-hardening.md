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

Harden the canonical `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION` programme with independently reproduced V2 safety findings while preserving PR #1135's canonical-programme plus thin-Ultra-overlay composition. Limit the change to the canonical programme, the Ultra evaluation, the minimum Ultra delta and this controlled task record.

Trusted-base reconstruction: `blakinio/Oteryn-Platform@fcafc20bc9705ca92256fdddc7433bcc3d191c40` (`main`, observed 2026-08-17).

Intended branch: `docs/oteryn-20260817-repository-migration-programme-hardening`.

## Acceptance criteria

- [x] Current root and `docs/agents` governance was read from the trusted base.
- [x] Live `main`, active tasks, open PRs and branch ownership were reconstructed before editing.
- [x] No active task or open PR owns the four changed paths.
- [x] The canonical programme remains the sole migration authority.
- [x] The Ultra file remains a thin execution overlay rather than a replacement mega-prompt.
- [x] Hardening covers authority narrowing, evidence leases, target collision, bounded risk acceptance, single-mutation transactions, rollback-versus-redirect, drift invalidation and docs-only completion semantics.
- [x] The Ultra evaluation covers positive, negative, boundary, drift, collision, rollback, unavailable-tooling and unreproduced-review cases.
- [x] Full local exact diff and deterministic structural checks pass for the reconstructed files.
- [ ] The intended branch exists in `blakinio/Oteryn-Platform` from the revalidated current `main`.
- [ ] Exactly one PR is open for this task and records the final exact head.
- [ ] Repository-required Agent Governance and CI checks pass on the exact PR head.
- [ ] Exact-head self-review, review-thread hygiene and source-branch lifecycle gates are terminal.

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
blockers:
  - current execution environment exposes no write-capable GitHub connector, authenticated Git transport or gh client
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T17:14:15Z
head: UNKNOWN
branch: docs/oteryn-20260817-repository-migration-programme-hardening
pr: none
status: blocked
context_routes:
  - agent-governance
  - prompt-as-code
  - ecosystem-repository-migration
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-programme-hardening.md
proven:
  - Live main was reconstructed at fcafc20bc9705ca92256fdddc7433bcc3d191c40 before the candidate diff was authored.
  - PR 1135 merged the canonical-programme plus thin-Ultra-overlay model and changed four governance files.
  - The two existing active tasks own no path changed by this candidate.
  - The four open PRs at reconstruction own no path changed by this candidate.
  - The candidate changes only the canonical programme, Ultra overlay, Ultra evaluation and this task record.
  - Local deterministic structural validation and full-diff inspection passed for the reconstructed candidate.
derived:
  - The candidate can be carried by one bounded documentation/governance PR after live-main and ownership drift are rechecked.
unknown:
  - The exact external critical-review V2 artifact was unavailable to this execution environment; only findings independently reproduced against the trusted-base files were included.
  - The named branch exists only in the reconstructed local worktree; its remote head, PR identity, required checks and review state are unavailable because repository write access is unavailable here.
conflicts: []
first_failure:
  marker: github-write-channel-unavailable
  evidence: gh is not installed; authenticated Git credentials/SSH keys and a write-capable GitHub connector are unavailable; direct git clone also failed DNS resolution.
rejected_hypotheses:
  - Replace the Ultra overlay with a self-contained migration mega-prompt.
  - Edit SHORT_PROGRAM_INVOCATIONS or unrelated programme/runtime files without a demonstrated need.
  - Treat GitHub redirects as rollback.
  - Claim that a remote branch, PR or CI result exists without direct verification.
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-programme-hardening.md
validation:
  - command: git diff --check
    result: PASS
    evidence: no whitespace errors in the reconstructed exact candidate diff
  - command: python3 /mnt/data/validate_oteryn_migration_hardening.py
    result: PASS
    evidence: 114/114 local structural assertions passed for versions, baseline links, thin-overlay constraints, scenario/invariant coverage, task schema and changed-path allowlist
  - command: full exact diff self-review
    result: PASS
    evidence: all four allowed paths in the local reconstructed exact diff were inspected with zero unresolved material findings
  - command: E2E
    result: NOT_APPLICABLE
    evidence: documentation and prompt-governance hardening only; no executable repository coordinate, runtime or control-plane mutation occurred
  - command: GitHub Actions Agent Governance and CI on exact PR head
    result: BLOCKED
    evidence: no write-capable GitHub channel exists to create the remote branch/PR and trigger exact-head gates
blockers:
  - A write-capable GitHub channel is required to create the intended branch, persist this task, open exactly one PR and run required exact-head gates.
next_action: Transfer the prepared candidate through an authenticated GitHub write channel onto the named branch created from revalidated current main.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: the candidate branch exists only in the reconstructed local worktree; remote task branch and PR could not be created in the current execution environment
source_branch_evidence: local branch docs/oteryn-20260817-repository-migration-programme-hardening; no authenticated write-capable GitHub channel available
```

## Notes

The candidate intentionally does not edit `docs/agents/SHORT_PROGRAM_INVOCATIONS.md`, programme runtime state, architecture ADRs or any non-Platform repository. The exact external V2 review text was not available, so no unreproduced claim was promoted into canonical governance.
