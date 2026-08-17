---
task_id: OTERYN-20260817-ecosystem-repository-migration-prompt
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - open PRs touching docs/agents/prompts or docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - active tasks owning prompt or short-command routing paths
optional_reads:
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
---

# OTERYN-20260817-ecosystem-repository-migration-prompt

## Goal

Persist the owner-requested high-reasoning Oteryn ecosystem repository-migration programme as one canonical prompt plus durable short alias `OTERYN-REPO-MIGRATION`, initial programme state and explicit manual prompt-evaluation matrix, without changing runtime/product behaviour or triggering owner-funded AI review.

## Acceptance criteria

- [x] Fresh protected `main` and prompt/governance contracts were read before mutation.
- [x] No open PR or active task was found owning the new prompt/short-command routing scope.
- [ ] `docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md` contains the complete bounded role, live-state, migration, safety, validation and final-response contract.
- [ ] `docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md` gives the alias a durable live-state entry point without caching transient SHAs as authority.
- [ ] `docs/agents/SHORT_PROGRAM_INVOCATIONS.md` registers `OTERYN-REPO-MIGRATION` and its canonical prompt/programme state.
- [ ] The prompt evaluation matrix records positive, negative, boundary, stale-state, injection, authority, Atlas-extraction and owner-funded-AI cases and explicitly does not claim automated model evaluation.
- [ ] Final changed paths are limited to the five declared documentation/governance paths.
- [ ] Exact-head documentation/governance CI and whole-diff self-review pass before merge eligibility.
- [ ] No Draft->Ready transition or other action consumes owner-funded Codex/OpenAI quota without exact separate authorization.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260817-ecosystem-repository-migration-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-prompt.md
modules:
  - agent-governance
  - prompt-routing
dependencies:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T08:55:00Z
head: UNKNOWN
branch: docs/oteryn-ecosystem-repository-migration-prompt
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260817-ecosystem-repository-migration-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-prompt.md
proven:
  - Protected main at task admission was f617120975cb1522cad87d74f8bea37f829b2b64.
  - Root AGENTS forbids owner-funded AI without exact use authorization and requires substantial work to use a task branch and task record.
  - PROMPTING_STANDARD v2.1 requires substantial autonomous prompts to declare run scope, live-state/trust, scope, acceptance, execution, outcome verification, closeout and stop conditions.
  - PROMPT_EVAL_STANDARD permits a documented manual scenario matrix when executable prompt-eval infrastructure is unavailable, but it must not be described as automated proof.
  - No active task record owns prompt/short-command routing paths at admission.
  - No open PR matching docs/agents/prompts or SHORT_PROGRAM_INVOCATIONS.md was found at admission.
derived:
  - OTERYN-REPO-MIGRATION is a sufficiently short unique owner alias while the full programme ID remains OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.
unknown:
  - Final exact branch head and PR number until the coherent documentation commit and Draft PR are created.
  - Whether repository automatic Codex review remains enabled at final Ready transition; no such transition is authorized by this task.
conflicts: []
first_failure:
  marker: premature-pr-create
  evidence: Two initial PR-create attempts against a not-yet-created branch returned GitHub 422 invalid head and made no repository change; branch creation then succeeded normally.
rejected_hypotheses:
  - Direct main write was rejected because root AGENTS requires the task-branch/PR workflow after bootstrap.
changed_paths: []
validation:
  - command: repository path/ownership preflight
    result: PASS
    evidence: live main, AGENTS, routing, prompt standard, prompt eval standard, active tasks and open PR overlap were inspected
  - command: runtime/component/browser E2E
    result: NOT_APPLICABLE
    evidence: prompt/programme/task/registry documentation only; no executable product/runtime behaviour changes
blockers:
  - none
next_action: Create one coherent five-path documentation commit, open a Draft PR, then inspect exact-head CI and whole diff without triggering owner-funded AI review.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active until the prompt registration delivery reaches an intentional terminal state
source_branch_evidence: pending
```

## Notes

This task registers the future migration programme only. It does not itself create, rename, transfer or extract any product repository.