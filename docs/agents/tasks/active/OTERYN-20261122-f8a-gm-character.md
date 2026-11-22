---
task_id: OTERYN-20261122-f8a-gm-character
governing_issue: 2002
required_reads:
  - docs/contracts/ADMIN_CONTROL_AGENT_WIRE_RATIFICATION.md
  - portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md
search_first: []
optional_reads: []
---

# F8a GM player console

Issue #2002 governs E2E coverage. The coordinator narrowed this packet to the
catalogued GM player-console reads and accepted commands. The dedicated branch
is `codex/feat-gm-character`, based on
`33145d6ec7662cd01f91e304ba09fa904edeaf37`. Retain it for coordinator review.
No PR was requested.

The independent review fixes bind sanction rows to the requested character when
the result supplies an ID, localize known sanction values, and require explicit
retry for refused dossier and child queries. Browser coverage includes a senior
GM without MFA, result isolation between two MFA-verified senior GMs, signed
sanction mismatch, nullable sanction ID, and dossier and inventory retries.

- Qualify search, dossier, inventory and sanctions in both languages and viewport
  sizes through the authenticated Admin Agent.
- Refuse a dossier whose verified payload names a different character.
- Preserve the contract gate for staff commands.

Owned paths are the admin console projection and the GM browser spec and
harness. The existing ADM framing is ratified. The pinned staff-actions v1
profile is pending joint acceptance and lacks a staff result union. The five
staff command definitions remain disabled. The accepted MUTE/JAIL wire has a
separate Support enforcement implementation, outside this GM read scenario.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T16:21:47Z
head: 75dd880336700456d24a4cf5d9f91cb98ab3edd4
branch: codex/feat-gm-character
pr: none
status: ready
context_routes:
  - admin console
  - browser qualification
owned_paths:
  - portal-rs/crates/admin/src/console
  - portal-rs/e2e/admin-agent.mjs
  - portal-rs/e2e/harness.mjs
  - portal-rs/e2e/specs/gm-player-console.spec.mjs
proven:
  - The ADM transport framing is ratified.
  - The staff-actions profile awaits joint acceptance.
  - The four browser projects passed against a disposable PostgreSQL 18.6 harness.
derived:
  - The GM browser harness may advertise the existing player reads.
unknown: []
conflicts: []
first_failure:
  marker: dossier target mismatch
  evidence: The previous projection accepted a result naming a different character than the signed request target.
rejected_hypotheses:
  - A staff catalogue entry alone authorizes execution.
changed_paths:
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/projection.rs
  - portal-rs/e2e/admin-agent.mjs
  - portal-rs/e2e/harness.mjs
  - portal-rs/e2e/specs/gm-player-console.spec.mjs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/ENVIRONMENT.md
validation:
  - command: cargo build --locked -p oteryn-portal
    result: PASS
    evidence: Built the portal binary exercised by the harness.
  - command: cargo test --locked -p oteryn-admin --lib
    result: FAIL
    evidence: PostgreSQL 18.6, 231 passed and three unrelated Bazaar or Commerce concurrency tests timed out during the browser run. All console tests passed.
  - command: cargo test --locked -p oteryn-admin --lib with exact filters for the three timed-out cases
    result: PASS
    evidence: Both Bazaar cases and the Commerce case passed exact reruns after the browser harness stopped.
  - command: Playwright gm-player-console.spec.mjs
    result: PASS
    evidence: pl-1440, pl-390, en-1440 and en-390 each passed on the first attempt. No runner errors.
blockers: []
next_action: Coordinator reviews the retained branch evidence for Issue #2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the GM player console evidence, with no PR requested.
source_branch_evidence: Published and verified on origin as codex/feat-gm-character.
```
