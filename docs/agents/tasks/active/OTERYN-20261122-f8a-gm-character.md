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
`33145d6ec7662cd01f91e304ba09fa904edeaf37` and merged with `origin/main` at
`eaa6a2702a6f09505b4e58314e2211f721c7da1b`. Retain it for coordinator review.
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

The follow-up review fixes select the same latest request for display and retry
admission, verify result epochs separately, corroborate account-scoped sanctions
against the live LCFA portfolio, and share the single-live-agent eligibility
rule. Real mTLS and browser tests cover reconnects, account mismatch, foreign
retries and sanction permission revocation. A PostgreSQL test covers a second
agent connecting after the retry form appears.

The transfer review adds a staff-only read of the current LCFA owner, hides
sanctions when ownership cannot be resolved, and serializes rollup liveness
with retry admission. Browser tests cover a suspended owner, overlapping
snapshots, a mismatched account reference and both permitted and revoked
sanction retries. The route matrix tracks the CSRF-protected retry POST.

The result review requires LCFA completion coverage for account-scoped
sanctions and holds the control ownership fence through result verification.
The operations dashboard reports unverified successes as unavailable.
Browser coverage checks the watermark boundary and matches dossier children
by operation.

Owned paths include the admin console and control channel, LCFA ownership
lookup, and GM browser tests and harness. The existing ADM framing is ratified.
The pinned staff-actions v1
profile is pending joint acceptance and lacks a staff result union. The five
staff command definitions remain disabled. The accepted MUTE/JAIL wire has a
separate Support enforcement implementation, outside this GM read scenario.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T20:37:30Z
head: 7510db3891a3d1ae5cb62b382b9bd8cb72102c59
branch: codex/feat-gm-character
pr: none
status: ready
context_routes:
  - admin console
  - browser qualification
owned_paths:
  - portal-rs/crates/admin/src/console
  - portal-rs/crates/admin/src/control
  - portal-rs/crates/characters/src/lcfa
  - portal-rs/e2e/admin-agent.mjs
  - portal-rs/e2e/harness.mjs
  - portal-rs/e2e/inventory.py
  - portal-rs/e2e/inventory.json
  - portal-rs/e2e/route-expectations.json
  - portal-rs/e2e/specs/gm-player-console.spec.mjs
proven:
  - The ADM transport framing is ratified.
  - The staff-actions profile awaits joint acceptance.
  - The four browser projects passed against a disposable PostgreSQL 18.6 harness.
  - The committed retry matrix admits POST only and expects refusal without CSRF.
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
  - portal-rs/crates/admin/src/console/access.en.ftl
  - portal-rs/crates/admin/src/console/access.pl.ftl
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/projection.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/console/templates/read.html
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/characters/src/lcfa/store.rs
  - portal-rs/crates/characters/src/lcfa/tests.rs
  - portal-rs/e2e/admin-agent.mjs
  - portal-rs/e2e/harness.mjs
  - portal-rs/e2e/inventory.py
  - portal-rs/e2e/inventory.json
  - portal-rs/e2e/route-expectations.json
  - portal-rs/e2e/specs/gm-player-console.spec.mjs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/ENVIRONMENT.md
validation:
  - command: cargo build --locked -p oteryn-portal
    result: PASS
    evidence: Built the portal binary exercised by the harness.
  - command: cargo test --locked --no-fail-fast -p oteryn-admin -p oteryn-characters --lib
    result: PASS
    evidence: PostgreSQL 18.6 and Rust 1.100.0 passed 235 admin and 121 characters tests.
  - command: Playwright gm-player-console.spec.mjs
    result: PASS
    evidence: pl-1440, pl-390, en-1440 and en-390 passed with zero runner errors. The harness exited cleanly.
blockers: []
next_action: Coordinator reviews the retained branch evidence for Issue #2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the GM player console evidence, with no PR requested.
source_branch_evidence: Published and verified on origin as codex/feat-gm-character.
```
