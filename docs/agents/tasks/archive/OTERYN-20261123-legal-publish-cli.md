---
task_id: OTERYN-20261123-legal-publish-cli
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261123-legal-publish-cli

## Goal

Enable a host operator to publish audited, final-approved legal editions
into an existing portal database before activating the release gated by
signup readiness. Governing issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002).

## Acceptance criteria

- [x] The portal command and staff CMS share validation, save, immutable history and audit.
- [x] The command requires an active staff account with both page permissions and refuses placeholder text.
- [x] PostgreSQL tests cover bilingual readiness, source order, authorization, immutability and audit attribution.
- [x] The deployment and page documentation describe the operator sequence and host-root MFA boundary.

## Ownership

```yaml
owned_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/crates/content/src/pages.rs
  - portal-rs/crates/content/src/pages/staff.rs
  - portal-rs/crates/content/src/pages/tests.rs
  - portal-rs/crates/content/src/pages/README.md
  - deploy/oracle/README.md
  - docs/agents/tasks/active/OTERYN-20261123-legal-publish-cli.md
modules:
  - portal binary
  - content pages
  - Oracle operator documentation
dependencies:
  - Existing editorial pages, legal version and audit schemas
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-23T08:28:13Z
head: 4b86c79691926636f2a33ea17e27c3f000b4a32c
branch: codex/legal-publish-cli
pr: 2051
status: completed
context_routes:
  - CMS legal editions and Oracle deployment
owned_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/crates/content/src/pages.rs
  - portal-rs/crates/content/src/pages/staff.rs
  - portal-rs/crates/content/src/pages/tests.rs
  - portal-rs/crates/content/src/pages/README.md
  - deploy/oracle/README.md
  - docs/agents/tasks/active/OTERYN-20261123-legal-publish-cli.md
proven:
  - GitHub issue 2002 is open, and the requested base matches origin/main.
  - The staff CMS and signup readiness paths use existing legal snapshot tables.
  - The owner approved the public re-export and classified the absent generator gate as not applicable.
  - The Compose override selected the candidate image, retained the service environment, and forwarded the UTF-8 body through standard input in a disposable check.
  - The scratch binary passed the real publish-legal-edition and check-signup-legal commands against an isolated PostgreSQL schema.
derived: []
unknown:
  - Jira synchronization for issue 2002.
conflicts: []
first_failure:
  marker: Operator entry point absent on the base.
  evidence: All six PostgreSQL regressions failed with the unimplemented publisher before the shared save was enabled.
rejected_hypotheses: []
changed_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/crates/content/src/pages.rs
  - portal-rs/crates/content/src/pages/staff.rs
  - portal-rs/crates/content/src/pages/tests.rs
  - portal-rs/crates/content/src/pages/README.md
  - deploy/oracle/README.md
  - docs/agents/tasks/active/OTERYN-20261123-legal-publish-cli.md
validation:
  - command: cargo fmt --check
    result: PASS
    evidence: Repository Rust 1.97 formatter accepted the staged Rust sources.
  - command: cargo test -p oteryn-content
    result: PASS
    evidence: PostgreSQL 18.6 passed 36 unit tests and seven integration tests, including all six new legal operator cases and the unchanged staff CMS tests.
  - command: cargo clippy -p oteryn-content -p oteryn-portal --all-targets -- -D warnings
    result: PASS
    evidence: Strict Clippy passed on Rust 1.97 in a scratch copy with the proposed public re-export.
  - command: cargo test -p oteryn-portal --bin oteryn-portal
    result: PASS
    evidence: 267 passed, zero failed and one was ignored in the scratch copy with the proposed public re-export and a disposable PostgreSQL 18.6 instance configured with more lock capacity.
  - command: publish-legal-edition and check-signup-legal smoke test
    result: PASS
    evidence: Signup readiness refused missing editions and passed after four publishes. Publications worked in all four supported environments and attributed all seven successful saves.
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: No increase in enforced categories.
  - command: python3 portal-rs/e2e/mutation/generate.py --check
    result: NOT_APPLICABLE
    evidence: The script is absent on main, and the task changes neither E2E nor generated files.
blockers: []
next_action: none; delivered by PR #2051 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Branch requested for owner review under issue 2002.
source_branch_evidence: pending
```
