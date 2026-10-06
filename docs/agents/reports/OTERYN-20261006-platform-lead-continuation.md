# Oteryn Platform Lead — continuation snapshot (2026-10-06)

This file is a continuation aid only. Live GitHub/Jira state, repository instructions and current protected `main` remain authoritative.

## Protected baseline at snapshot

- Repository: `Oteryn/Oteryn-Platform`.
- Protected `main`: `a834ef54b154c48b119da20c3bdb6fca74178750`.
- Main commit at snapshot: `docs(agents): archive completed native preprod ops task (#1470)`.
- No Oteryn-Game or Oteryn-Atlas mutation is carried forward by this snapshot.
- No production/protected-environment mutation, real secret/certificate issuance or deployment was performed by this Platform Lead pass.

## Work completed in this pass

### LiveOps WorldStatus + configured Maintenance — Issue #1458 / PR #1459

Terminal state: **MERGED / COMPLETE**.

Verified delivery:

- PR #1459 merged from exact head `a41b2210e3c433176f3843be5758d22def63386d`;
- merge commit: `73138cb10bdea17a1127363cbaa299177c0593e1`;
- Issue #1458 is closed;
- public Today now consumes a public-safe `App\\LiveOps\\WorldStatus` projection over canonical WorldId/ChannelId and native runtime-status evidence;
- configured maintenance/login policy remains distinct from observed runtime readiness;
- stale/unavailable/invalid/degraded evidence remains explicit and does not fabricate offline;
- public output omits GameNode identity, assignment/fencing generations, route revisions and private endpoints;
- browser acceptance covered ready/stale/unavailable/maintenance/recovery;
- final implementation fixes included Pint formatting and PHPStan nullability/dead-branch cleanup.

Do not reopen #1458 unless live evidence shows a regression.

### Synology autostart reconciliation — Issue #1339 / PR #1461

State: **REPOSITORY CANDIDATE VALIDATED; LIVE HOST PROOF STILL REQUIRED**.

Current exact PR head:

- PR #1461: `bb90f1098675c076ab570fe13080f45983e46dc4`;
- branch: `fix/1339-synology-autostart-org-runner`.

Repository root cause and repair:

- legacy repository runner/container `oteryn-synology-staging` had already been retired;
- stale workflow still required `oteryn-deploy-runner/runner`;
- repair now targets current organization-runner Compose project/service `oteryn-organization-runners/platform`;
- six persistent Platform staging services remain fail-closed with restart-policy checks;
- transient `tls-init` and Atlas/Game organization runners remain outside Platform mutation;
- focused pre-PR contract readback passed 20/20 assertions.

Exact-head standard CI on PR #1461 is green:

- Agent Governance PASS;
- CI PASS;
- CodeQL PASS;
- Edge Security PASS;
- Phase 7 PASS;
- Platform DB Outage PASS;
- Game Auth Ticket Concurrency PASS;
- Synology Production Target Preflight PASS;
- Synology Rollback Contract PASS.

Do **not** claim Issue #1339 closed until separately authorized real Synology host execution/readback proves the corrected workflow on the protected host.

### Premium physical Platform↔Game E2E — Issue #1431 / PR #1462

State: **STANDARD CI GREEN; PHYSICAL CROSS-REPO E2E NOT YET PROVEN**.

Current exact PR head:

- PR #1462: `46f06a33d862aa972f1e84c375014d1de1fc3ca5`;
- branch: `test/1431-premium-physical-e2e`.

Prepared harness:

- real Platform Premium snapshot producer in ephemeral PHP/MariaDB;
- real TLS 1.3 mutual-TLS nginx terminator;
- read-only checkout of Oteryn-Game pinned to `0fceef1f2cc311ed21eda96d168e1f8b2b298f83`;
- real Game `PremiumSnapshotClient` plus `snapshot::validate`;
- intended cases: disabled 503, unknown AccountId 404, NONE, ACTIVE, REVOKED;
- no Game write, no staging/production mutation and no real certificate issuance.

Exact-head standard CI on #1462 is green:

- Agent Governance PASS;
- CI PASS;
- CodeQL PASS;
- Edge Security PASS;
- Phase 7 PASS;
- Platform DB Outage PASS;
- Game Auth Ticket Concurrency PASS.

Important: the custom `Premium Physical Cross-Repository E2E` workflow is **not present among the recorded head runs at this snapshot**. Therefore physical producer→consumer proof must still be run and pass before PREM-E2E closeout is claimed.

### Native login N4-P / LCFA reconciliation — Issue #1419

Current state was reconciled against Platform and Game protected main.

Already integrated Platform slices:

- contract #1420;
- native ticket redemption #1423;
- runtime-status ingestion #1425;
- scope-assignment ingestion #1426;
- issuer + Registry/runtime route selection #1427;
- Go Gateway protocol-v2 forwarding #1429.

Game-side `LCFA-1 / ListCharactersForAccount` producer is complete.

Remaining release-safe Platform gap:

- Platform `ListCharactersForAccount` ingestion/read model;
- private snapshot + watermark routes;
- dedicated projection-purpose mTLS identity;
- monotonic epoch/revision + watermark freshness;
- issuer §5.4 character ownership check replacing D171 unverified mode for release-safe flow;
- public `native-characters` read must reuse read-only OAuth/native-client verification and must not call ticket issuance because issuance revokes the bearer token.

The previous ownership overlap with #1459's `config/game-auth.php` is now released because #1459 is merged. Fresh-read current main/ownership before claiming the LCFA paths.

### GameCatalog native persistence — Issue #489 / architecture escalation #1460

Verified state:

- Game now has a real native content-tree inventory and locked export-v1 producer contract/tooling;
- Platform PR #1229 already validates the native export envelope but deliberately persists nothing;
- existing `game_catalog_snapshots` remains Legacy Canary Compatibility-shaped and cannot truthfully store native snapshots using dummy Canary fields.

Architecture escalation #1460 is open and labeled `agent:ready` / `programme:platform`.

Blocking question:

- choose the durable native persistence composition that reuses one GameCatalog candidate/profile/activation lifecycle without inheriting Canary identifiers/persistence semantics or creating a second co-authoritative catalogue.

Do not implement native persistence until #1460 decides this boundary.

### Platform API / observability / public edge — Issue #490

Re-audit disposition:

- general Platform API is terminally **DEFERRED** by ADR 0036 until a named consumer exists;
- OperationsObservability repository/staging architecture and evidence contract are terminal;
- residual #490 scope is live PublicEdge/protected-environment proof plus direct production topology/observability/backup/restore evidence.

Do not start speculative general Platform API implementation from the stale original tracker text.

## Jira / programme state

Primary programme Jira item: `KAN-41`.

This snapshot should be mirrored to KAN-41 with:

- #1459 merged/closed;
- #1461 repository CI green but live-host verification still pending;
- #1462 standard CI green but physical Premium workflow not yet proven;
- #1419 LCFA consumer now path-unblocked by #1459 merge, subject to fresh ownership scan;
- #1460 architecture decision pending;
- #489/#490 tracker corrections above.

## Resume priority

1. Fresh-read current protected `main`, open PRs and active task ownership.
2. For #1462, run/repair the actual `Premium Physical Cross-Repository E2E` workflow and only then classify PREM-E2E complete.
3. For #1461, preserve repository-green state; request separately authorized real-host verification rather than mutating Synology from the Platform Lead alias.
4. Start the Platform LCFA consumer slice under #1419 only after a fresh path-ownership check; the previous #1459 overlap is gone.
5. Let Platform Architecture Review resolve #1460 before native GameCatalog persistence work.
6. Continue autonomous Platform-only work on disjoint paths; escalate Game-owned changes to Game coordinator/architect rather than writing Oteryn-Game.

