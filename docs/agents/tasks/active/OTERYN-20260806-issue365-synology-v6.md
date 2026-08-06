---
task_id: OTERYN-20260806-issue365-synology-v6
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 735
parent_issue: 365
branch: validation/issue365-synology-v6-20260806
pull_request: pending
status: implementing
task_kind: validation
implementation_authorized: true
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
---

# OTERYN-20260806-issue365-synology-v6

## Goal

Execute the exact approved PHP 8.5 validator once with the complete proven environment contract, retain terminal evidence and close the observation PR without merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:47:00Z
head: resolved-from-live-branch
branch: validation/issue365-synology-v6-20260806
pr: pending
status: implementing
context_routes:
  - testing
  - frontend-ux
  - ci-repair
owned_paths:
  - .github/ISSUE365_SYNOLOGY_V6_VALIDATION_ONLY.md
  - .github/workflows/issue365-synology-v6.yml
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v6.md
proven:
  - validator artifact 8964153679 passed immutable generation, structural patch, Bash syntax and all internal hashes
  - environment proof artifact 8964791387 passed exact historical-source extraction with unresolved inputs empty
  - canonical explicit inputs are TARGET_SHA, RUNBOOK_REF, PLAYWRIGHT_IMAGE and GH_TOKEN from secrets.GITHUB_TOKEN
  - automatic GITHUB-prefixed inputs are supplied by Actions
  - prior v5 failure occurred before image build solely because RUNBOOK_REF was absent
unknown:
  - whether complete-contract validator reaches PHP 8.5 wrapper and browser matrix
  - terminal product or technical matrix result
conflicts: []
first_failure:
  marker: missing-validator-environment-contract
  evidence: run 31093534539 stopped on missing RUNBOOK_REF; static proof #733 resolved the complete contract
changed_paths:
  - .github/ISSUE365_SYNOLOGY_V6_VALIDATION_ONLY.md
  - docs/agents/tasks/active/OTERYN-20260806-issue365-synology-v6.md
validation:
  - command: static validator proof 31092791643
    result: PASS
    evidence: artifact 8964153679
  - command: environment contract proof 31094295511
    result: PASS
    evidence: artifact 8964791387; unresolved inputs empty
  - command: complete-contract one-shot runtime
    result: NOT_RUN
    evidence: workflow will be created last as single trigger
blockers: []
next_action: Open draft observation PR, create one-job workflow last, classify its single terminal run and close without merge.
```
