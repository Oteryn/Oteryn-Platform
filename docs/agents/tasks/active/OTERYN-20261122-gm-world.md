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
updated_at: 2026-11-22T15:30:30Z
head: f3e40067d77a47a959b1efaf01b1bab3b3e8c812
branch: codex/feat-gm-world
pr: none
status: ready
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
  - Both browser scenarios passed in pl-1440, pl-390, en-1440 and en-390.
derived: []
unknown:
  - Acceptance evidence for an analytics.catalog result-body schema is unavailable.
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
    evidence: 231 passed in the initial admin run. Fourteen later focused cases passed. The existing contention test passed in isolation after timing out during resource contention.
  - command: cargo test --locked -p oteryn-admin -p oteryn-content -p oteryn-bazaar --lib --no-fail-fast -- events::tests moderation::tests
    result: PASS
    evidence: 15 Bazaar moderation and 6 CMS event tests passed on PostgreSQL 18.6.
  - command: Playwright gm-world and gm-bazaar specs in all four configured projects
    result: PASS
    evidence: 8 passed. Browser cases covered unavailable ops reads, editorial transitions, moderation audit, independent approval and reconciled cancellation.
  - command: cargo fmt --all --check
    result: PASS
    evidence: workspace formatting check passed.
blockers: []
next_action: Retain the qualified branch for review under issue 2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the corrected scope and coverage evidence.
source_branch_evidence: GitHub readback confirmed the qualified implementation and fixtures at f3e40067d77a47a959b1efaf01b1bab3b3e8c812.
```
