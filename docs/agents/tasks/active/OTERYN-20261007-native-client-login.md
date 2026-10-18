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

Current state: validating. Actual Windows client completed OAuth, received a native ticket from separate first-party Rust registration, and received a Gateway admission grant. The retained test server has one Game-created character and accepted account-character snapshot/watermark. Expanded Platform native ticket/admission tests passed 82 tests/865 assertions; Rust OAuth CLI passed 4/16. Affected Pint and PHPStan passed. Actual Game admission remains unavailable before authorization: character RootUnavailable or evidence Custody; Game #1927 is diagnosing without loosening accepted deadlines or source age. Separate Rust registration selects native ticket kind from authenticated client only; Canary client preserved. Native issuance remains default-off and testing/preproduction-only. No immutable candidate freeze, PR or independent review yet.
Jira mapping pending; no programme mutation.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-08T05:35:00Z
head: UNKNOWN
branch: fix/native-client-login-20261007
pr: none
status: validating
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
  - docs/agents/tasks/active/OTERYN-20261007-native-client-login.md
proven:
  - Windows verified TLS and HTTP 200 for all three portal stylesheets and both login images after Game nginx repair.
  - Real Game bootstrap committed one character and Platform accepted its ownership projection.
  - Chromium blocked the native loopback redirect under the original form-action policy.
  - Registered Rust client OAuth produced a bearer, native ticket and Gateway grant on the actual Windows client flow.
derived:
  - Separate first-party Rust OAuth registration preserves Canary client semantics and follows contract section 4.1 client-selected ticket kind.
unknown:
  - End-to-end native admission with the new native ticket issuer.
conflicts: []
first_failure:
  marker: CANARY_TICKET_ISSUER_ON_NATIVE_FLOW
  evidence: Real Rust OAuth bearer reached an issuer requiring IdentityCanaryAccount; no native ticket existed.
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
validation:
  - command: PHPUnit focused directory, OAuth, ticket, character and security-header tests
    result: PASS
    evidence: Expanded native ticket/admission suite 82 tests/865 assertions, Rust client CLI 4/16 passed; earlier CSP-stage 50/521 passed.
  - command: PHPStan affected CSP and directory PHP files, memory limit 512M
    result: PASS
    evidence: Zero errors; first physical attempt exhausted image default 128M and is retained in NAS evidence log.
blockers:
  - none
next_action: Diagnose Game durability unavailability, then freeze the bounded Platform candidate and obtain applicable independent exact-head review.
```
