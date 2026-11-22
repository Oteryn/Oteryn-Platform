---
task_id: OTERYN-20261122-f22a-puzzles-publication
governing_issue: 2002
required_reads:
  - portal-rs/crates/minigames/src/puzzles/README.md
  - portal-rs/e2e/README.md
  - portal-rs/e2e/ENVIRONMENT.md
search_first: []
optional_reads: []
---

# F22a daily puzzle publication

The operator CLI imports and activates an editorial puzzle set on its UTC date,
with the permission, publication identifier, immutable puzzle rows, and audit
evidence checked at the Platform boundary.

## Acceptance criteria

- Publish from a validated local JSON file under the existing default-off puzzle switch.
- Roll back the publication on partial failure and replay a publication ID without duplicate rows or audit records.
- Exercise publication and the localized player flow in four Playwright projects.
- Check for a Game puzzle contract before adding any transport.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T14:45:00Z
head: 33145d6ec7662cd01f91e304ba09fa904edeaf37
branch: codex/feat-puzzles-ingress
pr: none
status: ready
context_routes:
  - admin-rbac
  - database
  - testing
owned_paths:
  - portal-rs/bin/oteryn-portal/src/puzzles_cli.rs
  - portal-rs/crates/minigames/src/puzzles/
  - portal-rs/migrations/20261122150200_puzzles_publication.sql
  - portal-rs/e2e/specs/puzzles.spec.mjs
proven:
  - The Platform daily read model uses local immutable rows and the approved world read model.
  - The Platform contract registry contains no puzzle Game wire contract.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: PostgreSQL shared lock table exhausted
  evidence: SQLSTATE 53200 in the provisioned test instance. Rerun passed on a disposable PostgreSQL 18.6 instance with max_locks_per_transaction=1024.
rejected_hypotheses: []
changed_paths:
  - portal-rs/
validation:
  - command: git diff --check
    result: PASS
    evidence: Working tree diff is whitespace-clean.
  - command: cargo test --locked -p oteryn-minigames --lib
    result: PASS
    evidence: 82 passed against disposable PostgreSQL 18.6.
  - command: cargo test --locked -p oteryn-admin --lib rbac::tests::permission_catalog_is_stable_unique_and_covered -- --exact
    result: PASS
    evidence: The new permission is assigned to the intended built-in role.
  - command: Playwright specs/puzzles.spec.mjs
    result: PASS
    evidence: pl-1440, pl-390, en-1440 and en-390 passed against the local browser launcher.
blockers: []
next_action: Publish the qualified candidate on the requested branch for issue 2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: The owner requested branch delivery for review under issue 2002 without opening a PR.
source_branch_evidence: codex/feat-puzzles-ingress
```
