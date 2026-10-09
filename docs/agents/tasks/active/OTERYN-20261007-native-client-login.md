---
task_id: OTERYN-20261007-native-client-login
governing_issue: 1473
required_reads: []
search_first: []
optional_reads: []
---

# Native client login availability repair

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1473

Owner explicitly authorized Platform and Game client repairs on 2026-10-07.
Branch: `fix/native-client-login-20261007`; one writer in this session.
Owned paths: `app/Http/Controllers/GameAuth/ClientDirectoryController.php`, `bootstrap/app.php`, `tests/Feature/GameAuth/ClientDirectoryTest.php`, this packet.

The native client checks anonymous world availability before OAuth. The missing `/v1/client/directory` returns 404 and blocks login. Return only canonical Registry worlds/channels with fresh ready runtime evidence and matching authorized routes. Anonymous characters remain empty; owner data uses existing authenticated API. No credentials or private route data escape. Native production activation remains refused, consistent with existing route authority.

Acceptance: focused PHP feature tests; client parser compatibility; actual test OAuth/ticket/Gateway/admission path with Game #1927. No production deployment or protected integration requested.

Current state: authoring on draft PR #1484. On 2026-10-09 the actual Windows native client completed browser PKCE, owner character selection and admitted gameplay using a fresh local test identity and Game-created character. Separate Rust registration selects native ticket kind from the authenticated client; Canary client semantics remain separate. The remembered-device extension is still an OFF contract candidate; its HTTP candidate source is qualified in isolated tests but remains default OFF and inactive. No production deployment, accepted successor contract, immutable candidate freeze or exact-head independent security approval is claimed.

Historical 2026-10-07/08 validation: the expanded Platform native ticket/admission suite passed 82 tests/865 assertions; Rust OAuth CLI passed 4/16; affected Pint and PHPStan passed. The earlier actual-flow RootUnavailable/Custody failure was resolved by the later retained local-runtime recovery, without weakening ownership, evidence deadlines or source age. These historical runner results do not validate the HTTP adapter now being authored.
Jira mapping pending; no programme mutation.

## Local homepage native status repair 2026-10-08

Owner reports homepage Unavailable and asks to connect the Rust runtime. Confirmed HomePageQuery read Canary configured channels; actual container probe failed with QueryException/PDOException. Reuse existing public-safe LiveOps PublicWorldStatusQuery for native Registry worlds, preserving its policy and freshness labels and Canary fallback when no native worlds are configured. Extend HomeWorldSummary with optional native public summaries; production home hero/realm render their translated readiness states without fabricating a player count (native reports do not supply population). Additional owned paths: app/PublicPortal/HomePageQuery.php, app/PublicPortal/ViewModels/HomeWorldSummary.php, resources/views/home.blade.php, resources/views/game/partials/native-home-worlds.blade.php, tests/Feature/LiveOps/PublicWorldStatusQueryTest.php.

Added real native fixtures covering homepage fresh readiness without a Canary database or synthetic population and expired evidence remaining stale. Isolated PHP8.5.6 Docker test environment uses SQLite memory, not runtime databases. Direct PHPUnit HomeTest5/45 and PublicWorldStatusQueryTest7/30 PASS; affected Pint and PHPStan PASS, git diff check PASS. Installed changed source/views into local Docker Desktop Platform container and cleared compiled views/reloaded PHP-FPM. Actual Windows homepage HTTP200, both hero/realm native state=ready, old generic unavailable summary absent. Docker Desktop migrated runtime native admission had already passed. Public game/servers, rankings and other Canary-backed pages are not converted by this homepage change. No production deployment, source authority bypass, frozen candidate or independent review claimed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-09T10:23:05Z
head: 549de370bbb00e389f0c49fa2b780be623056806
branch: fix/native-client-login-20261007
pr: 1484
status: implementing
context_routes:
  - docs/agents/BUILD_TEST_MATRIX.md
owned_paths:
  - app/Console/Commands/EnsureNativeOAuthClient.php
  - app/GameAuth/OAuth/NativeOAuthClientManager.php
  - app/GameAuth/OAuth/VerifiedNativeOAuthAccess.php
  - app/GameAuth/OAuth/VerifyNativeOAuthAccess.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/NativeLogin/NativeGameLoginTickets.php
  - app/Http/Middleware/SecurityHeaders.php
  - app/Http/Controllers/GameAuth/ClientDirectoryController.php
  - bootstrap/app.php
  - tests/Feature/GameAuth/OAuth/NativeOAuthClientManagerTest.php
  - tests/Feature/GameAuth/OAuth/NativeOAuthNativeTicketTest.php
  - tests/Feature/GameAuth/OAuth/NativeOAuthPkceTest.php
  - tests/Feature/GameAuth/ClientDirectoryTest.php
  - app/PublicPortal/HomePageQuery.php
  - app/PublicPortal/ViewModels/HomeWorldSummary.php
  - resources/views/home.blade.php
  - resources/views/game/partials/native-home-worlds.blade.php
  - tests/Feature/LiveOps/PublicWorldStatusQueryTest.php
  - docs/agents/tasks/active/OTERYN-20261007-native-client-login.md
proven:
  - Windows verified TLS and HTTP 200 for all three portal stylesheets and both login images after Game nginx repair.
  - Real Game bootstrap committed one character and Platform accepted its ownership projection.
  - Chromium blocked the native loopback redirect under the original form-action policy.
  - Registered Rust client OAuth produced a bearer, native ticket and Gateway grant on the actual Windows client flow.
  - On 2026-10-09 the actual Windows client completed browser PKCE, owner character selection and admitted gameplay using a normally registered local test identity and fenced Game character bootstrap.
  - Draft PR 1484 is open on fix/native-client-login-20261007 at observed remote head 549de370bbb00e389f0c49fa2b780be623056806; governing Issue 1473 remains open.
  - The disabled remembered-device service and existing Identity MFA suites passed 41 tests and 315 assertions in isolated owner-PC Docker SQLite memory; affected Pint passed.
derived:
  - Separate first-party Rust OAuth registration preserves Canary client semantics and follows contract section 4.1 client-selected ticket kind.
unknown:
  - Concrete client lock/journal/HTTPS integration and accepted successor contract review.
  - Accepted owning remembered-device contract, deployment-database concurrency and complete secure-vault client integration.
  - Exact-head independent security review and repository-wide lifecycle checks after publication of the successor candidate.
conflicts: []
first_failure:
  marker: ACTIVE_TASK_CHECKPOINT_METADATA
  evidence: Remote candidate 549de370 failed classification and Agent Governance because the new candidate packet lacked a Context checkpoint and this packet omitted live draft PR 1484; these are scoped authoring metadata repairs.
rejected_hypotheses:
  - Disabling TLS or character ownership verification is unnecessary.
changed_paths:
  - app/Console/Commands/EnsureNativeOAuthClient.php
  - app/GameAuth/OAuth/
  - app/GameAuth/NativeLogin/NativeGameLoginTickets.php
  - app/Http/Middleware/SecurityHeaders.php
  - app/Http/Controllers/GameAuth/ClientDirectoryController.php
  - bootstrap/app.php
  - tests/Feature/GameAuth/OAuth/
  - tests/Feature/GameAuth/ClientDirectoryTest.php
  - app/PublicPortal/HomePageQuery.php
  - app/PublicPortal/ViewModels/HomeWorldSummary.php
  - resources/views/home.blade.php
  - resources/views/game/partials/native-home-worlds.blade.php
  - tests/Feature/LiveOps/PublicWorldStatusQueryTest.php
  - docs/agents/tasks/active/OTERYN-20261007-native-client-login.md
validation:
  - command: Repository checkpoint validator for both owned packets; bounded task_liveness.evaluate_task against live GitHub
    result: PASS
    evidence: Both Context checkpoints validate against contract v1 and both owned task records report DRAFT_PR with no findings for live PR 1484; this does not clear unrelated repository-wide CI findings.
  - command: PHPUnit focused directory, OAuth, ticket, character and security-header tests
    result: PASS
    evidence: Expanded native ticket/admission suite 82 tests/865 assertions, Rust client CLI 4/16 passed; earlier CSP-stage 50/521 passed.
  - command: PHPStan affected CSP and directory PHP files, memory limit 512M
    result: PASS
    evidence: Zero errors; first physical attempt exhausted image default 128M and is retained in NAS evidence log.
  - command: Actual Windows browser PKCE, native owner character selection and Game admission
    result: PASS
    evidence: Completed local admission on 2026-10-09 using normally registered test identity, fenced Game bootstrap and renewed local runtime PKI; no password or database reset.
  - command: Isolated owner-PC DeviceSessions and existing Identity MFA suites; affected Pint
    result: PASS
    evidence: 41 tests and 315 assertions passed in Docker SQLite memory, including real MFA confirmation invalidating prior OAuth and device credentials; no runtime database migration.
  - command: Isolated HTTP/core/MFA/cache suites; scoped Pint and PHPStan level10
    result: PASS
    evidence: 64 tests and657 assertions; original OAuth access/refresh consumption and remembered continuation verified in memory fixtures. Candidate OFF.
  - command: Complete remembered-device native client integration
    result: NOT_RUN
    evidence: Concrete exclusive lock, durable journal, HTTPS and monotonic-ticket adapters remain unwired.
blockers:
  - Complete native client integration and applicable contract/security qualification remain outstanding.
  - Existing unrelated LCFA task lifecycle and eight trusted-main unexplained branch findings remain outside these two scoped task-record repairs.
next_action: Publish the qualified disabled HTTP successor, preserving draft status and accurate lifecycle evidence, then finish native consumer bindings.
```


Latest local qualification (2026-10-09): fresh test identity/character created through normal registration and fenced Game ops. Actual Windows native client completed browser PKCE, owner character selection and admitted gameplay. Runtime PKI renewed locally without password/database reset. Remembered-device extension remains an OFF candidate with no HTTP/client activation; see OTERYN-20261009-remembered-device-candidate.md. The checkpoint head records the observed remote authoring anchor, not an immutable freeze of ongoing local successor source.
