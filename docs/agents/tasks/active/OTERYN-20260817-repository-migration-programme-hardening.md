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

Harden the canonical `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION` programme while preserving PR #1135's canonical-programme plus thin-Ultra-overlay composition. Resolve the post-green critical-review findings without widening the four-path scope or performing any physical repository/runtime mutation.

Trusted-base reconstruction: `blakinio/Oteryn-Platform@fcafc20bc9705ca92256fdddc7433bcc3d191c40` (`main`).

Branch: `docs/oteryn-20260817-repository-migration-programme-hardening`.
PR: `#1138`.

## Acceptance criteria

- [x] Current root and `docs/agents` governance read from the trusted base.
- [x] Live `main`, active tasks, open PRs and branch ownership reconstructed before editing.
- [x] Exactly one PR exists for this task: #1138.
- [x] Current PR changes only the four owned paths.
- [x] Canonical programme remains the sole migration authority.
- [x] Ultra remains a thin execution overlay and no longer defines a competing Tier-2 gate schema.
- [x] `READY_TO_EXECUTE` is separated from public `CUTOVER_READY` semantics.
- [x] Canonical transaction includes interruption recovery, replay guard, cutover lock, pre/post state and rollback-feasibility proof.
- [x] Evidence precedence and recovered/reproduced V2 concerns are recorded without treating chat/history as authority.
- [x] First compatibility exercise is dry-run/Tier-0/1 only; absent stochastic harness is not a behavioural PASS.
- [ ] Full exact-head self-review on the repaired final diff has zero unresolved material findings.
- [ ] Repository-required Agent Governance and CI pass on the repaired unchanged final head.
- [ ] Review-thread hygiene is terminal.
- [ ] Source-branch lifecycle and task archival are terminal after merge/closeout.

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
checkpoint_version: 1
updated_at: 2026-08-18T04:30:00Z
head: 5f2a0f09b51e94637adc1294cfa3fdd0a3c9d645
branch: docs/oteryn-20260817-repository-migration-programme-hardening
pr: 1138
status: implementing
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
  - Live main remained fcafc20bc9705ca92256fdddc7433bcc3d191c40 through the latest pre-repair drift check.
  - PR #1135 established the canonical-programme plus thin-Ultra-overlay model.
  - PR #1138 exact head 5f2a0f09b51e94637adc1294cfa3fdd0a3c9d645 passed Agent Governance run 32071180188 (#7073) and CI run 32071180184 (#7450).
  - Review inspection on PR #1138 returned zero submitted reviews and zero inline review threads before this repair cycle.
  - Post-green critical review found semantic issues not exercised by deterministic CI: executable readiness versus CUTOVER_READY, duplicated transaction gates, mutation-interruption recovery, compatibility dry-run and stale task evidence.
  - The repaired candidate keeps exactly the same four owned paths and performs no physical repository coordinate, runtime, deployment, credential, secret or live-game mutation.
  - File-library search did not surface an exact standalone V2 review artifact; recovered prior-review concerns were independently reproduced before incorporation.
derived:
  - Documentation/prompt-governance E2E remains NOT_APPLICABLE because no executable repository/control-plane effect is changed by this PR.
  - The prior green exact-head gates become historical evidence only after the repaired candidate changes head; fresh gates are required.
unknown:
  - Stochastic model/runtime adherence for the repaired candidate remains unmeasured because no executable repeated-trial harness exists for this prompt surface.
conflicts: []
first_failure:
  marker: post-green-semantic-review-findings
  evidence: Exact-head CI/Governance were green on 5f2a0f09..., but critical review identified contract-level ambiguities outside the deterministic text/checkpoint gate coverage.
rejected_hypotheses:
  - Green CI proves the 31+ behavioural scenarios; the repository evaluator explicitly performs deterministic text checks with zero model trials.
  - CUTOVER_READY can be an intermediate executable state; canonical semantics reserve it for exactly one unsupported or owner-only physical action.
  - Ultra should maintain a second cutover_gate schema; the canonical migration_transaction must be the single source of truth.
  - A timed-out physical mutation can be safely retried without resulting-state inspection; non-idempotent replay is fail-closed.
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-programme-hardening.md
validation:
  - command: Agent Governance run 32071180188 on 5f2a0f09b51e94637adc1294cfa3fdd0a3c9d645
    result: PASS
    evidence: historical pre-repair exact head passed; superseded for repaired candidate once head changes.
  - command: CI run 32071180184 on 5f2a0f09b51e94637adc1294cfa3fdd0a3c9d645
    result: PASS
    evidence: historical pre-repair exact head passed; superseded for repaired candidate once head changes.
  - command: executable migration E2E
    result: NOT_APPLICABLE
    evidence: documentation and prompt-governance hardening only; no executable repository/control-plane effect changed.
  - command: repaired exact-diff structural/self-review
    result: NOT_RUN
    evidence: candidate must be persisted first, then exact changed paths and full diff re-inspected.
  - command: repaired exact-head Agent Governance and CI
    result: NOT_RUN
    evidence: fresh pull_request runs are required after the repair commit changes the head.
blockers: []
next_action: Persist the four-path repair to PR #1138, inspect the exact new diff, then run/observe Agent Governance and CI on the unchanged repaired head.
```

## Self-review

```yaml
self_review:
  result: FAIL
  exact_head: 5f2a0f09b51e94637adc1294cfa3fdd0a3c9d645
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings:
    - executable readiness and CUTOVER_READY semantics were conflated
    - Ultra duplicated canonical Tier-2 gate state
    - transaction interruption/replay recovery was underspecified
    - behavioural evaluation evidence was not distinguished strongly enough from deterministic CI
    - task checkpoint/PR narrative lagged the actual green exact-head runs
  evidence:
    - PR #1138 exact diff and per-file patch inspection
    - Agent Governance #7073 PASS and CI #7450 PASS on 5f2a0f09...
    - repository PROMPT_EVAL_STANDARD and deterministic prompt_eval.py limitations
```

This self-review remains `FAIL` until the repaired exact head is persisted and re-reviewed; do not carry the previous PASS across the material prompt change.

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: branch is active for PR #1138 and remains under repair/validation
source_branch_evidence: docs/oteryn-20260817-repository-migration-programme-hardening
```

## Notes

The candidate intentionally does not edit `docs/agents/SHORT_PROGRAM_INVOCATIONS.md`, programme runtime state, architecture ADRs or any non-Platform repository. Recovered V2 findings are treated only as review leads and are incorporated only where independently reproducible against current canonical files.
