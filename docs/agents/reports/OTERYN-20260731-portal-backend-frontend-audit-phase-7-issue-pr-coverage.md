# Phase 7 — live Issue, PR and ownership reconciliation

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Scope: audit and evidence only  
Observed main: `39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c`  
Audit head before this phase: `dee81b96943fc5a426b400b5e443325fede2f9b2`

## Purpose

The Phase 6 delivery crosswalk maps 18 product and operational modules plus 43 benchmark capabilities, but it does not provide a fail-closed map of the current GitHub work graph. Phase 7 reconciles every currently open Issue and pull request to a module, role, task and intentional disposition so a separate implementation agent cannot silently omit, duplicate or mis-sequence work. It also checks whether the current workflow set can actually validate the live audit head.

No product code, workflow, deployment, production state, issue lifecycle or external repository was changed.

## Live inventory

At observation time the repository had:

- 21 open Issues;
- 6 open pull requests;
- 5 open pull requests carrying active task records;
- 1 temporary validator pull request without a standalone task record;
- 0 active tasks declared by `docs/agents/ACTIVE_WORK.md`.

All 21 Issues and all 6 pull requests are represented in `phase-7-issue-pr-coverage.json`.

## Pull-request disposition

| PR | Task / Issue | Current role | Required disposition |
|---:|---|---|---|
| `#338` | Game Catalog schema 1.3 consumer / `#330` | blocked required consumer | keep draft until pinned Canary producer compatibility exists |
| `#381` | this audit / `#326`, `#365`, `#451` | audit evidence | keep draft while exact-frozen validation, CI compatibility and material findings remain open |
| `#391` | official Linux client live-reference | blocked external-client interoperability research | keep draft, but add explicit programme Issue and module ownership |
| `#405` | production gate / `#91` | blocked required production evidence | keep draft until exact production prerequisites exist |
| `#471` | payment event core / `#470`, `#321`, `#278`, `#451` | active backend producer | keep draft and refresh its stale checkpoint before further implementation claims |
| `#476` | temporary Issue `#365` validator | non-mergeable observation channel | close without merge after run `30763456046` becomes terminal and evidence is persisted |

No pull request is classified as safe to merge or close during this audit phase.

## Issue-to-module coverage

The live issue graph covers:

- programme and exhaustive acceptance: `#451`, `#326`;
- Wiki/editorial evidence defect: `#365`;
- production operations and public edge: `#91`;
- character lifecycle and prerequisites: `#277`, `#317`, `#319`, `#320`, `#323`, `#324`, `#344`;
- commerce, payments and entitlements: `#278`, `#321`, `#322`, `#325`, `#470`;
- Game Catalog and optional knowledge: `#301`, `#302`, `#330`;
- portal/admin presentation follow-up: `#244`;
- scheduled stability and soak evidence: `#114`.

This mapping supplements rather than replaces the 18-module Phase 6 crosswalk. Product status remains unchanged.

## Finding — OTERYN-AUDIT-P7-001

**Severity: MEDIUM — OPEN**

The live coordination index and task checkpoints do not match the current pull-request graph.

Direct evidence:

- current `docs/agents/ACTIVE_WORK.md` declares no active tasks;
- open PRs are `#338`, `#381`, `#391`, `#405`, `#471` and `#476`;
- PR `#338` live head is `8baec8d66c1bab0b618684096300ab491dacacb4`, while its task records `b1adb5355871cc7ede579799669d38ca323e3dcc`;
- PR `#391` live head is `630ed73c09242cf3d37f3652b06fa252c6b0f10d`, while its task records `cabad487a139aaf0983dfc55cfb18d9f43720633`;
- PR `#405` live head is `6357fce7d68cfaa16452e7d71719a5c0ea886717`, while its task records `90f367963ddaee6fa6884319fc8cc54e23ca8ec4`;
- PR `#471` live head is `cda564d4072f8ddac9f258a106b660a3558c50d5`, while its task still records `head: UNKNOWN` and `pr: none`.

Impact: an autonomous coordinator can read stale ownership, blocker, validation or next-action state and duplicate work or execute it in the wrong dependency order.

Disposition: programme `#451` should refresh the active-work index and each affected task checkpoint from live Git state. PR `#381` records the finding only.

## Finding — OTERYN-AUDIT-P7-002

**Severity: MEDIUM — OPEN**

PR `#391` and task `OTERYN-20260801-official-linux-client-live-reference` have no explicit parent Issue and no first-class module in the production-completion ledger.

The work is not merely generic testing: it creates a repository capability for official-client Linux observation and plans Oteryn/OTClient/Canary compatibility requirements. The current 18-module ledger contains Platform API and Quality/E2E, but neither explicitly owns this external-client interoperability boundary.

Impact: the workstream can remain outside programme prioritization, dependency accounting, acceptance inventory and terminal closeout.

Disposition: programme `#451` should either adopt a first-class `external_client_interoperability` boundary or explicitly map PR `#391` to existing modules with one parent Issue and acceptance contract. PR `#381` performs no implementation or lifecycle mutation.

## Finding — OTERYN-AUDIT-P7-003

**Severity: MEDIUM — OPEN**

The current change-routing workflow rollout is not backward-compatible with pull-request heads created before the classifier files were introduced.

Exact-head evidence on `475013aa05a44a24d83cea09b0237147216c8d1f`:

- Agent Governance run `30767823565` passed after the audit checkpoint was repaired to the version-1 contract;
- CI run `30767823552` failed in `classify-changes` because `tests/ci/test_classify_changes.py` is absent from the exact PR head;
- Edge Security run `30767823563` failed for the same missing test file;
- Platform DB Outage run `30767823557` failed for the same missing test file;
- Game Auth Ticket Concurrency run `30767823549` failed for the same missing test file;
- Phase 7 Production-Like run `30767823551` failed because `scripts/ci/classify_changes.py` is absent from the exact PR head.

Both files exist on current main but not on the frozen-base audit branch. Each heavy workflow checks out the exact PR head and then executes classifier code from that checkout. Consequently, all five product-validation workflows stop before application setup or product tests.

Impact: a pre-rollout PR cannot obtain exact-head product validation under the current workflow definitions. The delivery contract cannot close without a backward-compatible classifier fallback, a controlled rebase, or an intentional import of CI support files.

Disposition: programme `#451` should assign this to the CI/governance implementation agent. PR `#381` does not modify workflows, committed tests or its frozen product baseline.

## Relationship to existing findings

These are coordination, taxonomy and CI compatibility findings in addition to the frozen portal/product findings:

- existing frozen portal/product findings remain `0 HIGH / 7 MEDIUM / 1 LOW`;
- Phase 7 adds `0 HIGH / 3 MEDIUM / 0 LOW` live work-graph and CI findings.

They do not change the 43-capability status counts or the Phase 6 policy-v2 result.

## Conclusion

The live GitHub work graph is exhaustively mapped for the current observation, and the exact-head workflow boundary has been tested. The mapping is evidence, not remediation. The audit remains `VALIDATED_WITH_CORRECTIONS` and `waiting` on the exact-frozen Issue `#365` run plus the externally owned CI compatibility correction. The implementation agent must use the Phase 6 module crosswalk together with the Phase 7 Issue/PR map rather than relying on `ACTIVE_WORK.md` alone.
