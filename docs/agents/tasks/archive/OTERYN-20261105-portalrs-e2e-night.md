---
task_id: OTERYN-20261105-portalrs-e2e-night
governing_issue: 530
required_reads:
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
search_first:
  - portal-rs
optional_reads: []
---

# Portal browser qualification

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (open at admission).
User authorizes local throwaway data, browser qualification, fixes, commits and normal push to the named branch; no PR.

## Goal and acceptance

Qualify every portal surface in Polish and English at 1440 and 390 pixels. Preserve security checks and document game-dependent pending/refusal results. Run locked build, format, clippy, PostgreSQL workspace tests, browser suite and token ratchet.

## Ownership

Owned: portal-rs/** and this packet. Baseline was fetched main bd9898c4; current main 5c4e5eb2 was normally merged at 743748a1, including the merged shared-shell UI. Jira synchronization unknown; no connector used. Shared environment services remain owned by environment bootstrap. Task-owned scratch: /workspace/portal-e2e-night; task databases: portal_e2e_night and portal_e2e_night_tests; local portal/worker/mail/proxy processes are ephemeral and stopped after qualification.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-06T03:36:59Z
head: 3180e9d48f008c0cee5eeadeb257648cde4cec1e
branch: codex/portal-rs-e2e-night
pr: none
status: blocked
context_routes:
  - testing
  - auth-identity
  - execution-resources
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portalrs-e2e-night.md
proven:
  - fresh main fetched and current main normally merged
  - ten production root causes fixed with owning regressions
  - 184 browser cases cover four language and viewport projects
  - governing Issue open
  - locked Rust tests 1041 passed plus all 3 opt-in browser cases passed
  - final browser matrix 156 passed and 28 failed with zero skipped or flaky
  - analyzed public HTML documents have zero serious or critical axe, overflow and focus findings
  - local task services stopped and disposable task databases removed
derived: []
unknown:
  - remaining production integration completion and unqualified positive lifecycles
  - Jira synchronization and mapping
conflicts: []
first_failure:
  marker: mandatory-browser-integration-gaps
  evidence: portal-rs/e2e/qualification.json
rejected_hypotheses: []
changed_paths: []
validation:
  - command: cargo build --locked -p oteryn-portal
    result: PASS
    evidence: portal-rs/e2e/REPORT.md
  - command: cargo fmt --all --check and cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: portal-rs/e2e/REPORT.md
  - command: cargo test --locked and cargo test --locked -- --ignored
    result: PASS
    evidence: portal-rs/e2e/REPORT.md
  - command: node run.mjs local-private-env.json test
    result: FAIL
    evidence: portal-rs/e2e/qualification.json
blockers:
  - missing browser modules and native authorization or authority adapters
  - unqualified positive editorial and owner-data lifecycles
next_action: implement the remaining configured security adapters and browser lifecycles listed in the report then rerun the four-project matrix
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Codex lane integration branch published without a PR under the owner's instructions; its work reached main through later portal-rs PRs and the branch belongs to the Codex lane, so this session does not delete it
source_branch_evidence: live branch lookup on 2026-11-11 shows refs/heads/codex/portal-rs-e2e-night present; archived 2026-11-11 after the owner made portal-rs the main platform
```

## Qualification record

See portal-rs/e2e/REPORT.md for root causes, acceptance summaries and the requested flow matrix. Acceptance remains incomplete until every mandatory row passes. No Jira mapping or programme mutation was invented.
