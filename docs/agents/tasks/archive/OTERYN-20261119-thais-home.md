---
task_id: OTERYN-20261119-thais-home
governing_issue: 1984
required_reads: []
search_first: []
optional_reads: []
---

# Publish the Thais homepage

Governing issue: https://github.com/Oteryn/Oteryn-Platform/issues/1984

Replace the localized homepage with the approved responsive design. Keep the existing portal session, mounted routes, world observations, news and rankings. Deliver the local artwork with the portal and use normal protected-main release controls.

## Acceptance criteria

- Both locales render the Thais hero, world and player metrics, five interactive professions, bestiary and getting-started guide.
- Mounted destinations, authenticated actions, unavailable data and local assets work through the Rust homepage.
- The exact reviewed release is delivered and verified at the public host.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-19T15:36:23Z
head: 19d2490b80e5acb30ddf2dea5c64c098a40d54a3
branch: feat/thais-portal-home
pr: 1985
terminal_pr_policy: archive_pending
status: completed
context_routes: []
owned_paths:
  - portal-rs/templates/pages/home.html
  - portal-rs/templates/components/thais-*.html
  - portal-rs/static/thais/
  - portal-rs/crates/ui/
  - portal-rs/crates/web/src/
  - portal-rs/crates/community/src/today/http.rs
  - portal-rs/crates/wiki/src/navigation.rs
  - portal-rs/e2e/
proven:
  - The public homepage is server-rendered by Askama. Its production data feed and account registry are reused.
derived: []
unknown:
  - Protected integration and production release outcome.
conflicts:
  - Draft PRs 1977 and 1799 are alternative layouts. The owner selected the Thais design; this branch does not modify either draft.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/
validation:
  - command: cargo test --locked -p oteryn-ui -p oteryn-web
    result: PASS
    evidence: PostgreSQL-backed Rust tests pass for localized rendering, account actions, runtime data and escaping.
  - command: Playwright homepage qualification
    result: PASS
    evidence: PL and EN desktop and mobile pass; authenticated mobile account and logout verified on the isolated portal.
blockers: []
next_action: none; delivered by PR #1985 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: One release candidate on its dedicated Platform branch.
source_branch_evidence: PR #1985 merged as 6f8038b355dde4d9007d4e7117670137f78188b8 and live branch lookup confirms refs/heads/feat/thais-portal-home is absent
```

Delivered by PR #1985 and archived.
