---
task_id: OTERYN-20261105-portalrs-local-e2e
governing_issue: 530
required_reads:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
search_first:
  - portal-rs/crates/identity
optional_reads: []
---

# Local portal-rs end-to-end qualification

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (live-read open).
Owner task in chat authorizes local test data, isolated portalrs containers and local fix commits only; forbids GitHub writes, publication and existing-stack changes.

## Goal and acceptance

Run all nine repository E2E specs in PL/EN at 1440/390 and real-container registration, verification, login/logout, MFA, recovery, abuse/CSRF and page reads. Produce `/home/mole/oteryn-cache/portalrs-e2e-r2-report.md`, a local fix patch and reproducible scripts under `portalrs-e2e/`; stop task containers at delivery.

## Ownership

Owned: this task record, bounded fixes within `portal-rs/`, external test scripts and `oteryn-testenv-portalrs*` resources only. One writer, local branch `portalrs-e2e-r2`; previous `portalrs-e2e-fixes` retained. Existing pkg6 packet is a ready integration record, not an active writer; live open PRs cover architecture draft and retained salvage, with no active implementation overlap found. No external repository inspection. Jira synchronization pending; this worker does not load Jira.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T12:45:00Z
head: 541af097dd9e0c0031afb4fee269050ac70096b3
branch: claude/portal-rs-e2e-r2
pr: 1763
status: ready
context_routes:
  - testing
  - auth-identity
  - execution-resources
owned_paths:
  - portal-rs/e2e/specs/account.spec.mjs
  - portal-rs/e2e/README.md
  - docs/agents/tasks/active/OTERYN-20261105-portalrs-local-e2e.md
proven:
  - live governing Issue 530 open; no overlapping implementation PR found
  - all nine specs and four projects covered by 240 final composite passes
  - real-container identity/mail/MFA/reset pass in PL and EN
  - twelve fixture-backed real-container page checks pass
  - no HTTP 500 or connection failure in final anonymous smoke
  - no GitHub write or push performed
derived:
  - zero-5xx qualification requires accepted sources beyond the seeded fixture families
unknown: []
conflicts: []
first_failure:
  marker: host SMTP EACCES then stale Bazaar refusal assertions
  evidence: external initial-smtp and full-suite results in portalrs-e2e
rejected_hypotheses:
  - successful Bazaar HTTP 200 is a production defect; package 21 supplies its real fixture transport
changed_paths:
  - portal-rs/e2e/specs/account.spec.mjs
  - portal-rs/e2e/README.md
  - docs/agents/tasks/active/OTERYN-20261105-portalrs-local-e2e.md
validation:
  - command: repository local.mjs and run.mjs all specs and projects plus focused account reruns
    result: PASS
    evidence: 240 final composite passes; all initial failures and reruns retained externally
  - command: real-container Playwright identity and page flows
    result: PASS
    evidence: 24 identity cases and twelve page checks; two broader account sweeps retain cold-source 503 findings
  - command: portalrs-smoke.sh final fixture configuration
    result: FAIL
    evidence: 22 of 134 requests return cold-source 503; no 500 or 000; gates retained
  - command: release build; cargo fmt --all --check; cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: external build-r2, fmt-r2 and clippy-r2 logs
  - command: RUST_TEST_THREADS=1 cargo test --locked with disposable PostgreSQL 18
    result: PASS
    evidence: 1094 passed; zero failed; three existing ignores; optional external Game fixture absent
blockers:
  - none
next_action: land PR 1763; accepted cold-data sources are needed before zero-5xx qualification
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: same-repository PR 1763 is open; disposition becomes terminal at merge
source_branch_evidence: PR 1763 for governing Issue 530
```

## First-run qualification outcome and recovery (historical)

Report: `/home/mole/oteryn-cache/portalrs-e2e-report.md`; reproducible scripts and sanitized evidence: `/home/mole/oteryn-cache/portalrs-e2e/`. All identity browser paths pass in both locales. Disabled account and public read surfaces still return their intentional 503s; no flags or gates were weakened. Registration mail delivery and stale registration-session verification failures are fixed with regression tests. The existing rate-limit rollover test now observes PostgreSQL's actual minute boundary; quota and independence assertions are unchanged. Initial unbounded DB lock-memory exhaustion and the reproduced timer failure are preserved as failed attempts in the report. Final bounded workspace run is green.

Gateway inspection was read-only and environment names only. A second gateway can share the isolated portal network namespace and reach its loopback admission listener; successful admission remains unavailable until its owning identity and admission adapters replace `Unavailable`. No second gateway was started.

Portal, worker, Mailpit, original portal DB/network/data volume and final image are intentionally retained for the requester. Disposable validation container and intermediate image were removed. Anonymous checks volume `b70f3b1f2c32157f8c01d3bdaedb2cee7c5e33f2194ed9877eb65d1d2a2b1efe` was implicitly created by PostgreSQL; its provenance is retained in `checks-volume-provenance.json`. The user's prefix-only resource rule prevents its deletion without an exact exception. Approval or explicit retention was requested asynchronously and is pending. No out-of-prefix removal was attempted. The reproduction helper now explicitly mounts a prefix-named test volume.

The checkpoint head is captured before the final test/evidence commit; the external report records all final commit SHAs. No GitHub write, push, PR, integration or programme completion is authorized or claimed. Testing and report delivery are complete; resource closeout remains waiting for the exact volume decision.

## Owner-authorized second run (2026-11-07)

Base `541af097dd9e0c0031afb4fee269050ac70096b3`; branch `portalrs-e2e-r2`, old branch retained. Live Issue 530 open; only open PRs 1678/1643 concern salvage/architecture, no active writer overlap. No GitHub writes or Jira connector use. Full nine-spec, four-project repository E2E plus container identity/public flows authorized. Outputs: `/home/mole/oteryn-cache/portalrs-e2e-r2-report.md`, fix patch if needed. All disposable keys and environment artifacts under `portalrs-env/` with umask 077; browser trace/screenshots/error-page capture disabled to avoid secret capture. Containers limited to exact task prefix; stop those started at closeout. Historical anonymous volume above remains outside scope. Jira sync pending.

Second-run safety deviation: the provided up helper ran one auto-removed unnamed migration container before being corrected to explicit `oteryn-testenv-portalrs-migrate`. No existing unrelated container was modified. The external report records this actual naming deviation and all environment/runner corrections.

## Second-run terminal evidence

Final repository E2E proof: 240 PASS / 0 FAIL / 0 SKIP, composed from the full nine-spec/four-project run and focused account reruns. Real identity flows, Mailpit verification/reset, TOTP and 12 fixture-backed page checks passed. The strict anonymous zero-5xx criterion remains FAIL: 22 cold-source 503s among 134 GETs; no 500 or connection failure. Two broad authenticated sweeps likewise preserve nine unavailable-source routes each. No runtime gate was weakened.

Release build, formatting, Clippy, JavaScript syntax, diff and checkpoint checks pass. Workspace tests report 1094 passed, zero failed, three existing ignores; the optional external Game import had no checkout and returned without importing. The test correction requires successful fixture-backed Bazaar reads plus exact missing/malformed-ID status, no-store, shell and CSRF evidence. Existing default-off/missing-authority Rust regressions passed.

All five prefix-named containers are stopped. Harness children/listeners are stopped; its current database pair and two retry databases were dropped, with zero task E2E databases left. Three disposable private harness scratch directories were removed. Persistent portal test data/configuration, named volumes/images/network and external sanitized evidence remain for the requester. The historical anonymous volume is outside this run and remains untouched. Jira synchronization remains pending; no publication or integration occurred.

The checkpoint head records the precommit base; the final local fix SHA and patch are recorded in `/home/mole/oteryn-cache/portalrs-e2e-r2-report.md`. The branch remains local for owner review. This records execution completion with explicit environment limitations, not zero-5xx qualification or production Game acceptance.
