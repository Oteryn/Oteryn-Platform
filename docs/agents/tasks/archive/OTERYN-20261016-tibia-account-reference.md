---
task_id: OTERYN-20261016-tibia-account-reference
governing_issue: 277
required_reads: []
search_first: []
optional_reads: []
---

# Tibia account reference draft

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/277.
Owner requested saving the completed read-only account reference as a draft.
Scope is documentation only; no product rule, runtime, schema or Game change.

## Acceptance criteria

- [x] Record observed account sections and accessible subforms.
- [x] Separate observation, manual evidence and unavailable state-dependent flows.
- [x] Exclude private account data and the withdrawn arbitrary policy proposal.
- [x] Publish as draft PR #1650 for owner review; do not merge or deploy.

## Ownership

```yaml
owned_paths:
  - docs/design/drafts/TIBIA_ACCOUNT_MANAGEMENT_REFERENCE_2026-10-16.md
  - docs/agents/tasks/active/OTERYN-20261016-tibia-account-reference.md
modules:
  - Accounts
  - CharacterProfiles
dependencies:
  - owner review of reference observations
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T08:30:00Z
head: 640f6615c22866e77f2536d64f2ffcb6f7f4fcad
branch: codex/draft-tibia-account-reference-20261016
pr: 1650
status: completed
context_routes:
  - web-cms
owned_paths:
  - docs/design/drafts/TIBIA_ACCOUNT_MANAGEMENT_REFERENCE_2026-10-16.md
  - docs/agents/tasks/active/OTERYN-20261016-tibia-account-reference.md
proven:
  - Owner authorized read-only inspection and draft publication.
  - Browser returned to account Overview without submitting forms.
derived:
  - The inventory is a product reference, not implementation acceptance.
unknown:
  - Owner selection of Oteryn product behavior from this reference.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - The withdrawn 24h rename and 7day deletion proposal is an accepted rule.
changed_paths:
  - docs/design/drafts/TIBIA_ACCOUNT_MANAGEMENT_REFERENCE_2026-10-16.md
  - docs/agents/tasks/active/OTERYN-20261016-tibia-account-reference.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261016-tibia-account-reference.md --require-checkpoint
    result: PASS
    evidence: Validated one checkpoint against contract v1.
  - command: product runtime tests
    result: NOT_APPLICABLE
    evidence: Documentation-only draft; no application, schema, workflow or runtime changes.
blockers: []
next_action: none; delivered by PR 1650 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path; head branch deleted after merge
source_branch_evidence: PR 1650 merged as 640f6615c22866e77f2536d64f2ffcb6f7f4fcad
```
