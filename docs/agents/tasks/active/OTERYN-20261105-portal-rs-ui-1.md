---
task_id: OTERYN-20261105-portal-rs-ui-1
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# UI-1 — shared portal shell

Owner's UI-1 instruction governs scope; open Platform programme parent: https://github.com/Oteryn/Oteryn-Platform/issues/530. Base: main c75f1cea. Dedicated branch: codex/portal-rs-ui-1. Push branch only, no PR, no workflow edits. Jira mapping unknown; no Jira synchronization attempted.

## Ownership

```yaml
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-rs-ui-1.md
modules:
  - shared-portal-shell
dependencies:
  - T39
blockers:
  - none
cross_repository_tasks:
  - none
```

## Implementation

Small `oteryn-ui` crate owns Chrome, registry, the existing shared Fluent catalogues and home/refusal rendering. Owning module crates register public entry routes from existing flags/mount prerequisites. Startup installs one outer Chrome extension adapter, reading existing browser sessions without writes or expiry refresh. Module page templates remain unchanged. Registration's existing route is the CTA; downloads exposes update APIs only and has no browser entry route. API-only modules, disabled configured adapters and routes requiring an unknown character/listing ID have no invented navigation destination. Existing form validation pages keep their field errors at 422; generic refusal responses use the shell.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T00:00:00Z
head: UNKNOWN
branch: codex/portal-rs-ui-1
pr: none
status: validating
context_routes: []
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-rs-ui-1.md
proven:
  - shared crate and module registration compile
  - focused shell tests pass
derived: []
unknown:
  - final workspace acceptance and browser evidence
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/**
validation:
  - command: cargo test -p oteryn-ui --lib
    result: PASS
    evidence: 3 focused tests passed
blockers:
  - none
next_action: complete strict workspace acceptance and live browser checks
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested a reviewable pushed branch without a PR; owner decides follow-up integration
source_branch_evidence: owner UI-1 instruction
```
