---
task_id: OTERYN-20261122-f22b-collection-adapters
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# F22b collection adapters

The owner scoped this branch to the collection read adapters and their focused
qualification under issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002).
The G-10 and G-11 wires are accepted. The candidate catalogue mapping limits
delivery to the flagged Platform evidence view.

## Acceptance criteria

- Prove current ownership before reading inventory or bestiary update metadata.
- Show unavailable with known update times or an unknown time in Polish and English.
- Qualify the real ingest and browser read in all four projects.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T16:20:00Z
head: 59b733b03c4d1c3f64235ea3e4e81fac6e181940
branch: codex/feat-collection-adapters
pr: 2014
status: completed
context_routes: []
owned_paths:
  - portal-rs/crates/minigames/src/collection/
  - portal-rs/crates/playerdata/src/authority/
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/templates/collection/evidence.html
  - portal-rs/e2e/
proven:
  - Base 33145d6ec7662cd01f91e304ba09fa904edeaf37.
derived:
  - The candidate mapping holds item and bestiary content unavailable.
unknown: []
conflicts: []
first_failure:
  marker: collection-selector-column-decoding
  evidence: The four-project rerun passed after switching the selector to row decoding.
rejected_hypotheses: []
changed_paths: []
validation:
  - command: cargo test --locked -p oteryn-playerdata -p oteryn-minigames --lib collection
    result: PASS
    evidence: PostgreSQL-backed tests passed, 17 in minigames and 1 in playerdata, including feed-lock contention.
  - command: node collection/run.mjs and node collection/run.mjs --read-disabled
    result: PASS
    evidence: All four projects passed in both profiles on separate databases, with journal ingest over mTLS.
  - command: Shared API default-off producer and OAuth PKCE browser specs
    result: PASS
    evidence: Default-off journal ingest returned 503 and OAuth passed in all four projects.
  - command: cargo fmt --all --check
    result: PASS
    evidence: Formatting clean.
blockers:
  - none
next_action: none; delivered by PR #2014 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository task branch for issue 2002.
source_branch_evidence: PR #2014 merged as 59b733b03c4d1c3f64235ea3e4e81fac6e181940 on 2026-11-22; refs/heads/codex/feat-collection-adapters no longer exists on origin
```
