# Oteryn Platform — repository audit inventory checkpoint

**Status:** `INCOMPLETE`  
**Date:** 2026-09-07  
**Repository:** `Oteryn/Oteryn-Platform`  
**Governing programme:** Issue #451  
**Audit PR:** #1294  
**Fixed inventory target (historical sections 1–5):** `main@294e18909b8319695021011ccbeb1386cac32ced`  
**Fixed inventory tree (historical sections 1–5):** `f8e7b29c94b5d8edd096cd18954ae105023d7e0f`  
**Current fixed audit generation (section 6 onward):** `3557085c20512d25576d8884cc54471665784b00`  
**Current fixed audit tree:** `67dd1ef42dc0f6b247fbc5538214ebe35bcc5352`

This checkpoint persists the next audit phase requested by the owner. It is a durable continuation record, not a final audit and not a production-readiness claim. Sections 1–5 preserve the earlier generation; the current continuation and its exact evidence boundary are recorded from section 6 onward.

## 1. Fixed-SHA inventory coordinate

**FACT.** GitHub live state resolved `main` to `294e18909b8319695021011ccbeb1386cac32ced`, tree `f8e7b29c94b5d8edd096cd18954ae105023d7e0f`. The branch endpoint reports `main` protected with required status context `platform-gate`.

**FACT.** PR #1294 remained an open draft immediately before this checkpoint, on branch `docs/20260906-comprehensive-platform-audit` at head `7f7fee8810c0e5d62fed3aa9478c09e7c8888839`.

## 2. Baseline-to-current drift reconciliation

**FACT.** GitHub compare from the original audit baseline `3b2ea1c7392187d5d22488673073dc8f8305a374` to the fixed inventory target `294e18909b8319695021011ccbeb1386cac32ced` reports exactly two commits and only these five changed paths:

- `docs/agents/AGENTS.md`;
- `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
- `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md`;
- `docs/agents/PROMPTING_STANDARD.md`;
- `tools/agents/policy_consistency.py`.

**DERIVED.** For any exact path outside those five files, repository content at `3b2ea1c7392187d5d22488673073dc8f8305a374` is unchanged at `294e18909b8319695021011ccbeb1386cac32ced`. Earlier direct evidence for such unchanged files may therefore be generation-reconciled to the fixed SHA without pretending the file was re-read in this phase.

**FACT.** All five changed governance paths were directly re-read at `main@294e18909b8319695021011ccbeb1386cac32ced`. Current governance removes fixed productive 60/120-minute worker runtime windows as stop authority while retaining bounded no-progress, retry, repair, command-timeout and terminal-CI controls.

## 3. Inventory enumeration progress

**FACT.** Recursive Git tree enumeration was performed against the fixed tree for major top-level repository families, including `.github`, `app`, `bootstrap`, `config`, `database`, `deploy`, `lang`, `ops`, `public`, `resources`, `routes`, `scripts`, `services`, `storage`, `tests`, `tools` and `docs`.

**FACT.** Additional segmented reads were performed for large documentation/governance families, including `docs/agents`, architecture, contracts, design, operations, testing, prompts, programmes, reports, active/archive task records and selected evidence/handover/eval trees.

**UNKNOWN.** The returned tree entries have not yet been consolidated into one canonical per-path ledger. Therefore exact tracked-file totals and exact `DIRECT / GROUPED / N/A / UNVERIFIED` totals remain unverified.

## 4. Current-main workflow revalidation completed in this phase

**FACT.** `.github/workflows/game-gateway-ci.yml` at the fixed SHA runs on `pull_request` and `push` path filters and has no `merge_group` trigger. It runs formatting, `go test ./...`, `go vet ./...` and a Go build.

**FACT.** `.github/workflows/game-auth-ticket-concurrency.yml` at the fixed SHA runs on `pull_request` only and has no `merge_group` trigger. Its MariaDB concurrency proof is conditionally selected by the repository change classifier.

**FACT.** `.github/workflows/build-synology-staging-images.yml` at the fixed SHA:

- omits `lang/**` from its pull-request/push path filters even though the Platform image build context is the repository root;
- defines `validate-deployment` and `build` as independent jobs; `build` has no `needs: validate-deployment` dependency;
- builds Platform, Game Gateway and deploy-runner images using `context: .`.

**DERIVED.** These observations revalidate the corresponding CI/build concerns from the earlier audit generation on the fixed current-main SHA because the workflow files are unchanged between the original baseline and the fixed SHA.

## 5. Scope truth

This phase materially improved inventory coverage, but it did **not** complete the owner's audit contract.

Still required before a truthful final report:

1. consolidate every tracked path on `main@294e18909b8319695021011ccbeb1386cac32ced` into one ledger;
2. assign each path exactly one disposition: `DIRECT`, `GROUPED`, `N/A` or `UNVERIFIED`;
3. finish the semantic review of every applicable workflow/build/test/instruction/governance family rather than treating filename enumeration as semantic coverage;
4. complete and normalize domains A-W;
5. run the independent cross-check for unassigned paths, duplicated entry points, stale authority and unsupported claims;
6. perform only the proportionate validation needed for the audit findings and distinguish executed tests from skipped/not-run tests.

**AUDIT_STATUS: `INCOMPLETE`.**

**Historical NEXT_ACTION:** build the canonical tracked-file ledger for `main@294e18909b8319695021011ccbeb1386cac32ced`, reconcile every enumerated path to `DIRECT / GROUPED / N/A / UNVERIFIED`, then continue the unresolved A-W domains from that ledger.

## 6. Current fixed generation and evidence reconciliation

**FACT.** The current audit generation is fixed at `3557085c20512d25576d8884cc54471665784b00`. GitHub's Git-commit response binds it to tree `67dd1ef42dc0f6b247fbc5538214ebe35bcc5352`. The existing PR was re-read at head `9cdb238aeabb9f7c88df66b54038159cbbd73bef` before this continuation write; it remained an open draft on the same authorized branch.

**FACT.** A fresh GitHub compare from `3b2ea1c7392187d5d22488673073dc8f8305a374` to this fixed generation reports three commits ahead, zero behind and eight changed paths: the five historical governance paths in section 2, plus `docs/agents/tasks/active/OTERYN-20260907-task-inventory-path-d26.md`, `tools/agents/task_issue_liveness.py` and `tools/agents/test_task_issue_liveness.py`.

**INFERENCE — high confidence.** Prior source-level evidence for paths outside these eight paths can be carried forward by content identity. This does not convert an old partial review into full per-path coverage, and does not carry forward mutable Issues, PRs, workflow runs, protection settings or deployment state.

## 7. F16 — P1 — runtime path changes can be classified as documentation-only

**Classification:** FACT + INFERENCE. Historical hypothesis `H01` is promoted to this finding for the reproduced cases; the historical identifier is retained as an alias, not counted as a second finding.

**Affected paths:** `scripts/ci/classify_changes.py`, `scripts/ci/required_test_gate.py`, `.github/workflows/ci.yml`, `tests/ci/fixtures/change-routing-cases.json`.

**FACT.** At fixed SHA `3557085c20512d25576d8884cc54471665784b00`, `list_changed_paths()` selects `git diff --name-only --diff-filter=ACMRD`. A Git type change (`T`) is excluded, and a detected rename exposes its destination without preserving the original runtime path for classification.

**FACT.** Actual temporary Git repositories were created, committed and compared using the fixed classifier. A regular-file-to-symlink change at `app/Identity/AuditFixture.php` together with a documentation edit produced only the documentation path in the classifier input. Moving that runtime file to `docs/AuditFixture.php` with a documentation edit produced `R100` in the full diff but only documentation destinations in the classifier input. Both cases returned `docs_only` and set all five classifier gates (`ci`, `phase7`, `edge`, `db_outage`, `game_auth_concurrency`) to false.

**FACT.** The fixed `required_test_gate.py` CLI was then executed with the resulting `ci=false`, successful classification and skipped runtime job. Both cases returned exit code zero and the message that runtime tests were `NOT_APPLICABLE`. Classification succeeded incorrectly, so the gate's failure-on-classification-error behavior did not catch either omission.

**Impact / INFERENCE — high confidence for CI selection.** Runtime removal or type changes can lose their required runtime classification. P1 prioritizes the verification-integrity risk. This is not evidence that an actual protected-branch merge bypass, production incident, malicious PR or deployment occurred; no such operation was attempted.

**Source identity verification:** local copies used for execution were checked using Git blob hashing, not merely filenames:

| Source | Fixed-generation blob SHA |
|---|---|
| `scripts/ci/classify_changes.py` | `8f194c0c0a9967d9dc8dc3fbe228984e56b8f82a` |
| `scripts/ci/required_test_gate.py` | `3d61ff97df7fdfed01bb2ef3104aeff7b7f4e3bf` |
| `tests/ci/test_required_test_gate.py` | `e32ee8c2655f6b3068cbf55d2e92d6d3ff45614d` |
| `tests/ci/fixtures/change-routing-cases.json` | `11a17e3f77cacf629281e39a8c50da3e81d99c95` |

**Verification actually performed:**

| Synthetic Git change | Runtime classification expected | Observed `ci` | Result against expectation |
|---|---|---|---|
| Type change only | true | true, through empty-diff fallback | PASS |
| Type change plus docs edit | true | false | FAIL |
| Ordinary runtime edit plus docs edit | true | true | PASS |
| Runtime deletion plus docs edit | true | true | PASS |
| Runtime-to-docs `R100` rename plus docs edit | true | false | FAIL |
| Runtime-to-runtime `R100` rename plus docs edit | true | true | PASS |
| Executable-mode change plus docs edit | true | true | PASS |
| Docs-only edit | false | false | PASS |

All 28 existing classifier policy fixtures passed. The eight synthetic Git routing cases produced six PASS and two FAIL. The eight tests in `RequiredTestGateTest` passed; the separate four workflow-contract tests were not included in that focused command. Eight classifier-output-to-gate CLI checks were executed; the two erroneous routing cases were accepted as `NOT_APPLICABLE`. The latter is reproduction of the finding, not a claim that runtime tests ran. Temporary repositories were isolated and removed after the probes. No product file, workflow or production state was changed.

**Reproduction boundary:** use the exact four source blobs above, create a temporary Git repository containing a regular `app/Identity/AuditFixture.php` and `docs/audit-fixture.md`, commit the base, make each change in its own case and commit the head, compare both `git diff --name-status --find-renames BASE HEAD` and the classifier's `list_changed_paths(BASE, HEAD)`, then classify those returned paths. Feed the resulting flag to `required_test_gate.py --classification-result success --ci-required false --runtime-tests-result skipped` for the two failing cases. An ordinary executable-bit change is a separate control; it is not the regular-file/symlink type-change case.

**RECOMMENDATION.** Repair the existing diff parser, not the audit architecture: preserve relevant type changes and both sides of renames/copies, and fail closed on unsupported/incomplete status input. Add real-Git regression fixtures for the two reproduced cases plus the passing controls. Remediation is not performed under this documentation-only audit authorization.

## 8. Delivery gap audit

**Audit delivery:** `INCOMPLETE`; F16 is a reproduced material finding, not completion of the whole repository audit. The historical reports and findings remain preserved. Every current claim above is bound to the fixed source generation or the explicitly identified live read.

**Material work still open:** consolidate the complete tracked-path ledger; finish every applicable workflow, authored-code and migration/provisioning review; reconcile all A-W domains and historical findings; inspect remaining accessible live governance and exact-head validation; complete the independent cross-check. None of those remaining obligations is marked PASS merely because this checkpoint exists.

**Implementation / deployment / product readiness:** no remediation implemented; no runtime, staging or production deployment performed; no production-readiness claim.

**Current NEXT_ACTION:** continue the full tracked-path reconciliation at `3557085c20512d25576d8884cc54471665784b00`, preserving the earlier semantic evidence and the reproduced F16 result; review remaining source families rather than restarting the audit.
