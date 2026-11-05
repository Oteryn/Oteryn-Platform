---
task_id: OTERYN-20261014-ci-flow-proposal-draft
governing_issue: 1515
required_reads: []
search_first: []
optional_reads:
  - docs/CI_FLOW_OPTIMIZATION_DRAFT_2026-10-14.md
---

# OTERYN-20261014-ci-flow-proposal-draft

## Goal

Governing GitHub Issue: #1515 (perf(ci): restore fast Platform CI) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Publish a draft docs-only CI flow optimization proposal and its measured audit CSV for independent review. No workflow, protection, deployment or runtime change is authorized.

## Acceptance criteria

- [ ] The proposal and audit CSV are reviewed independently before any implementation.
- [ ] Legacy token ratchet and the Platform gate pass on the PR head.

## Ownership

```yaml
owned_paths:
  - docs/CI_FLOW_OPTIMIZATION_DRAFT_2026-10-14.md
  - docs/CI_FLOW_AUDIT_2026-10-14.csv
  - docs/agents/tasks/active/OTERYN-20261014-ci-flow-proposal-draft.md
modules:
  - docs
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
updated_at: 2026-11-05T08:30:00Z
head: 1cf609696435aa2c9e2b73de678298ded9fb0a31
branch: codex/ci-flow-proposal-draft-20261014
pr: 1610
status: completed
context_routes:
  - docs
owned_paths:
  - docs/CI_FLOW_OPTIMIZATION_DRAFT_2026-10-14.md
  - docs/CI_FLOW_AUDIT_2026-10-14.csv
  - docs/agents/tasks/active/OTERYN-20261014-ci-flow-proposal-draft.md
proven:
  - Legacy-engine token in three quoted PR titles of the audit CSV is redacted so the legacy token ratchet passes.
derived: []
unknown:
  - Whether #1515 is the intended governing Issue (assumed as the closest open CI-speed Issue).
conflicts: []
first_failure:
  marker: legacy_token_ratchet docs +3
  evidence: classify-changes job on 951cc5da
rejected_hypotheses: []
changed_paths:
  - docs/CI_FLOW_OPTIMIZATION_DRAFT_2026-10-14.md
  - docs/CI_FLOW_AUDIT_2026-10-14.csv
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: docs=243 equals baseline
blockers:
  - none
next_action: none; delivered by PR 1610 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path; head branch deleted after merge
source_branch_evidence: PR 1610 merged as 1cf609696435aa2c9e2b73de678298ded9fb0a31
```

## Notes

Draft proposal for independent review. Audit CSV PR titles are redacted only for the legacy token ratchet; PR numbers keep correlation exact.
