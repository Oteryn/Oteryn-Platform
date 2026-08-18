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

Harden the canonical `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION` programme while preserving PR #1135's canonical-programme plus thin-Ultra-overlay composition. Resolve the post-green semantic review findings without widening the four-path delivery scope or performing a physical repository/runtime migration.

Current reconciled base: `blakinio/Oteryn-Platform@7798f4e8eb5453287444b7685b4e9fdd0e7eab20` (`main`).

Branch: `docs/oteryn-20260817-repository-migration-programme-hardening`.
PR: `#1138`.

## Acceptance criteria

- [x] Current root and `docs/agents` governance read from the trusted base.
- [x] Active tasks/open PRs checked for overlapping ownership.
- [x] Exactly one implementation PR exists for this hardening: #1138.
- [x] PR diff contains exactly the four owned paths.
- [x] Canonical programme remains the sole migration authority.
- [x] Ultra is reduced to a thin execution overlay and no longer defines a competing Tier-2 gate schema.
- [x] Internal `READY_TO_EXECUTE` is separated from public `CUTOVER_READY`.
- [x] Canonical transaction records pre/post state, cutover lock, replay guard, interruption recovery, point-of-no-return and rollback feasibility.
- [x] Evidence precedence and reproduced V2 concerns are recorded without promoting chat/history to authority.
- [x] First compatibility exercise is dry-run/Tier-0/1 only; unavailable stochastic trials remain `UNKNOWN`.
- [x] Main drift through Dynamic Semantic Atlas #1139/#1140 was reconciled; the new architecture is disjoint and preserves Game canonical authority / Atlas derived projection semantics.
- [x] Full four-file exact-diff self-review on reviewed content head `aafc7eebaabb5b90103a55187d4b94313bca3663` found zero unresolved material findings.
- [x] Agent Governance #7081 and CI #7458 passed on reviewed content head `aafc7eebaabb5b90103a55187d4b94313bca3663`.
- [x] Review submissions, inline threads and PR comments were empty at the final pre-readiness inspection.
- [ ] Final checkpoint-only recording commit has repository-selected checks green.
- [ ] PR reaches an intentional merge/closeout state and source branch lifecycle is reconciled.
- [ ] Task is archived after merge without violating the owner's one-implementation-PR scope.

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
  - main 7798f4e8eb5453287444b7685b4e9fdd0e7eab20
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T04:55:00Z
head: aafc7eebaabb5b90103a55187d4b94313bca3663
branch: docs/oteryn-20260817-repository-migration-programme-hardening
pr: 1138
status: ready
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
  - PR #1135 established the canonical-programme plus thin-Ultra-overlay model.
  - Post-green review findings were repaired in the canonical programme, thin overlay and evaluation record without adding changed paths.
  - Main advanced from fcafc20bc9705ca92256fdddc7433bcc3d191c40 to 7798f4e8eb5453287444b7685b4e9fdd0e7eab20 through Dynamic Semantic Atlas documentation; that drift changed none of the four owned paths and preserves Game canonical World/Content plus Atlas derived-projection authority.
  - Branch head aafc7eebaabb5b90103a55187d4b94313bca3663 contains current main as a parent and compares against current main with exactly the four owned paths.
  - Agent Governance run 32100538937 (#7081) and CI run 32100538888 (#7458) passed on aafc7eebaabb5b90103a55187d4b94313bca3663.
  - PR #1138 had zero submitted reviews, zero inline review threads and zero comments at final pre-readiness inspection.
  - No physical repository coordinate, runtime, deployment, credential, secret, production or live-game mutation is part of this PR.
  - The repository deterministic prompt evaluator does not execute an LLM; stochastic model/runtime adherence remains separately classified.
derived:
  - Runtime/migration E2E is NOT_APPLICABLE for this documentation/prompt-governance increment.
  - A checkpoint-only recording commit after aafc7eeb... does not change PROGRAM, Ultra or eval bytes but must still receive the path-selected exact-head checks before merge.
unknown:
  - Stochastic model/runtime adherence for the candidate remains unmeasured because no executable repeated-trial harness is available; first candidate exercise is fail-closed dry-run with Tier 2 disabled.
conflicts: []
first_failure:
  marker: post-green-semantic-review-findings
  evidence: Earlier green deterministic checks did not cover executable-readiness/CUTOVER_READY ambiguity, duplicated transaction gates, interruption replay recovery or behavioural-eval limitations; the repaired candidate now addresses these findings.
rejected_hypotheses:
  - Green deterministic CI proves stochastic behaviour; prompt_eval.py reports zero model trials.
  - CUTOVER_READY is an intermediate executable state; executable readiness is internal READY_TO_EXECUTE.
  - Ultra should define a second cutover gate schema; migration_transaction is canonical and singular.
  - A timed-out rename/transfer may be retried without state proof; replay is fail-closed.
  - Main drift required reauthoring this task; the two main commits were disjoint and their Atlas authority semantics are compatible with this candidate.
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-programme-hardening.md
validation:
  - command: GitHub compare main@7798f4e8eb5453287444b7685b4e9fdd0e7eab20 to aafc7eebaabb5b90103a55187d4b94313bca3663
    result: PASS
    evidence: exactly four changed files, all in the owned-path allowlist; branch contains current main as parent.
  - command: full exact PR patch review of PROGRAM, Ultra, eval and task candidate
    result: PASS
    evidence: canonical transaction/status/recovery semantics are singular and fail-closed; Ultra is thin; eval distinguishes static PASS from behavioural UNKNOWN.
  - command: Dynamic Semantic Atlas main-drift authority reconciliation
    result: PASS
    evidence: main documentation keeps Oteryn-Game/current Oteryn-v2 canonical World/Content and Atlas as a derived semantic projection; no owned-path conflict.
  - command: Agent Governance run 32100538937 (#7081) on aafc7eebaabb5b90103a55187d4b94313bca3663
    result: PASS
    evidence: exact reviewed content head passed repository governance validation.
  - command: CI run 32100538888 (#7458) on aafc7eebaabb5b90103a55187d4b94313bca3663
    result: PASS
    evidence: exact reviewed content head passed repository CI.
  - command: executable migration E2E
    result: NOT_APPLICABLE
    evidence: documentation and prompt-governance hardening only; no executable repository/control-plane mutation occurred.
  - command: PR review/thread/comment hygiene
    result: PASS
    evidence: zero reviews, zero inline review threads and zero PR comments at final pre-readiness inspection.
blockers: []
next_action: Persist this checkpoint-only readiness record, verify its exact-head Agent Governance and CI, then transition PR #1138 to Ready only if the no-owner-funded-AI safety check remains satisfied.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: aafc7eebaabb5b90103a55187d4b94313bca3663
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - exact per-file PR patches for all four owned paths
    - current-main compare and Dynamic Semantic Atlas authority reconciliation
    - Agent Governance #7081 PASS
    - CI #7458 PASS
    - zero PR reviews, threads and comments
```

The only following branch change is this checkpoint/readiness recording itself; it changes no programme/overlay/evaluation bytes and must still pass its own repository-selected checks before merge.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository documentation/governance task; delete source branch after successful squash merge
source_branch_evidence: PR #1138 branch docs/oteryn-20260817-repository-migration-programme-hardening
```

## Notes

The candidate intentionally does not edit `docs/agents/SHORT_PROGRAM_INVOCATIONS.md`, programme runtime state, architecture ADRs or any non-Platform repository. Recovered V2 findings are treated only as review leads and are incorporated only where independently reproducible against current canonical files.
