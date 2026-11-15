---
task_id: OTERYN-20261115-audit-bza
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Bazaar audit repairs

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open, verified 2026-11-15). Owner's task specifies the exact findings, base, branch, tests and push; no PR is requested. Single writer, no subagents. Existing active packet covers unrelated public-domain repair. Jira synchronization pending coordinator.

Owned paths: portal-rs Bazaar runtime, authority exchange, coin ports, character-command receipt parser and regression tests. Base d37a93d38e0175b3f12ba099e3cd0112a5457fac. Branch claude/happy-davinci-ijz3wy-fix-bza.

L2-04: base already has database publication barrier plus exact-byte legacy publication and terminal expiry retirement. Verify existing PostgreSQL regressions.
L2-07: extend terminal rejection reconciliation beyond expiry using the closed CC1 parser, immutable terminal evidence and a newly qualified binding.
L4-11: require verified terminal no-effect delivery evidence before authorized return, then authoritative returned custody before funds release. Preserve ambiguous fences and immutable operation/receipt history.

Status: ready for requested branch publication. Local PostgreSQL only; existing game/money flags retained; no contract edits or external game access. Validation and push evidence will be recorded before closeout.

Source branch disposition: retain for coordinator review of requested fix branch; no merge or PR requested.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T13:55:00Z
head: d37a93d38e0175b3f12ba099e3cd0112a5457fac
branch: claude/happy-davinci-ijz3wy-fix-bza
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/bazaar/src/game/
  - portal-rs/crates/characters/src/commands/mod.rs
  - portal-rs/migrations/20261115110034_fix_bza_compensation.sql
proven:
  - L2-04 already fixed by the base publication barrier and legacy exact-byte publication
  - L2-07 regression failed before repair and passes with all eleven non-expiry terminal rejection codes
  - L4-11 regression failed before repair and passes with audited return, coin refund, Gold release and ambiguous-evidence fences
  - Immutable operation history and existing default-off gates preserved; contract bytes unchanged
  - PostgreSQL regressions executed against an isolated local database; no production access
  - Repository identity confirmed as Oteryn/Oteryn-Platform
  - Rust 1.97 pin is installed; environment override removed for final validation without installing tools
  - Live workstream issue 536 is open; no task PR was requested or created
derived: []
unknown:
  - Coordinator Jira mapping and synchronization
conflicts: []
first_failure:
  marker: none
  evidence: expected before-fix regression failures retained locally
rejected_hypotheses:
  - L2-04 still uses the reviewed permanent source-revision refusal; the base has replaced it
changed_paths:
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/commands/mod.rs
  - portal-rs/migrations/20261115110034_fix_bza_compensation.sql
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97 repository pin
  - command: cargo clippy --locked --all-targets -p oteryn-bazaar -p oteryn-characters -- -D warnings
    result: PASS
    evidence: both touched crates, Rust 1.97 repository pin
  - command: cargo test --locked -p oteryn-bazaar -p oteryn-characters -- --nocapture --test-threads=1
    result: PASS
    evidence: 202 passed, zero failed or ignored; real PostgreSQL configured; barrier 0.069s, legacy publication 0.071s, rejection matrix 0.949s, return/refund 0.147s, expiry 0.099s, scheduling 0.161s
  - command: product browser E2E
    result: NOT_APPLICABLE
    evidence: internal durable saga repair; real PostgreSQL authority-to-wallet regressions exercise the changed actor-to-result path
blockers: []
next_action: Publish the owner-requested branch for coordinator review; no PR or merge authorized by this task
```

The checkpoint head names the admitted base. Validation covers the uncommitted candidate derived from it; the publication commit is reported by Git rather than embedding a self-referential SHA here. No exact-head PR gate is claimed: the owner explicitly requested a branch push without a PR.

Final focused PostgreSQL durations are measured by the test bodies (all cited tests exceed 0.05 seconds). Before-fix failures: ownership rejection retained the old operation (0.69s); audited delivery refusal blocked return publication (0.12s). A preliminary Rust 1.100 Clippy run hit a newer lint in untouched identity code; final checks use the installed repository pin and pass.
