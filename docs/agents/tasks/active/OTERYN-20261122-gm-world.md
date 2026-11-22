---
task_id: OTERYN-20261122-gm-world
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Ops console and Platform moderation

Governing issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002). The coordinator limited this packet to existing trusted ops reads, CMS events, and Platform Bazaar listing moderation. The branch is retained for coordinator review.

## Acceptance criteria

- Exercise the existing overview, world, node, database, migrations, ledger, policy, and connected-agent views using their current result paths.
- Require a reason for CMS event saves and state changes, independent Bazaar invalidation approval, and listing report decisions. Keep authorization and audit in the existing transactions.
- Check browser flows in Polish and English at both configured widths.

ADM-01 ratifies transport framing. The allowlisted `analytics.catalog` entry has no accepted result-body projection in the Platform repository. The accepted analytics feed specifies aggregate families. Bazaar changes concern the existing Platform moderation records and result presentation. Jira synchronization is unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T15:22:00Z
head: 33145d6ec7662cd01f91e304ba09fa904edeaf37
branch: codex/feat-gm-world
pr: none
status: validating
context_routes: []
owned_paths:
  - portal-rs/crates/content/src/events/
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/e2e/specs/gm-world.spec.mjs
  - portal-rs/e2e/specs/gm-bazaar.spec.mjs
  - portal-rs/e2e/staff-rbac-helpers.mjs
proven:
  - Live governing issue 2002 is open and owner correction defines the feature scope.
  - ADM-01 transport framing is ratified and existing ops reads use the control store.
  - The current portal binary built with the lockfile. The mTLS harness reached ready and seeded a disposable world on PostgreSQL 18.6.
  - The ops and CMS scenario passed in all four browser projects. The Polish desktop Bazaar scenario passed through reconciliation and cancellation.
derived: []
unknown:
  - Final browser results after fixture synchronization are pending.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/content/
  - portal-rs/crates/admin/
  - portal-rs/crates/bazaar/
  - portal-rs/templates/events/
  - portal-rs/templates/admin/events/
  - portal-rs/e2e/
validation:
  - command: cargo test --locked -p oteryn-admin --lib
    result: PASS
    evidence: 231 passed in the initial admin run. A later focused admin run passed 14 and reached the existing 10-second contention test deadline.
  - command: cargo test --locked -p oteryn-admin -p oteryn-content -p oteryn-bazaar --lib --no-fail-fast -- events::tests moderation::tests
    result: PASS
    evidence: 15 Bazaar moderation and 6 CMS event tests passed on PostgreSQL 18.6.
blockers: []
next_action: Verify the full four-project browser rerun.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the corrected scope and coverage evidence.
source_branch_evidence: GitHub readback confirmed the fixture commit 7d21e95c19b2bd583b4259806c834c4d9f2d1c8e.
```
