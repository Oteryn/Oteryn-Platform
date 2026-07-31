# Issue #365 static cause analysis — historical thumbnail HTTP 500 traffic

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Classification: `PROVEN` test-isolation defect with historical `CI_PROVEN` runtime evidence.

## Corrected conclusion

The historical Wiki thumbnail HTTP 500 responses are not evidence of spontaneous failure for valid EditorialMedia objects.

They are explained by deterministic acceptance-state leakage:

1. `admin-wiki-editorial-media.spec.mjs` seeds a valid EditorialMedia row.
2. The same test intentionally corrupts its stored objects and later removes them.
3. The database row remains present.
4. The spec has no EditorialMedia reset in `beforeEach` or `afterEach`.
5. Later browser projects query the global approved-media library and request thumbnails for those stale rows.
6. `WikiEditorialMediaFileResponse` detects missing or integrity-failed bytes and throws `WikiEditorialMediaUnavailable`.
7. The application returns HTTP 500 for that condition; the existing Editorial Media acceptance test explicitly expects this 500 response and verifies the accessible `Preview unavailable` fallback.

The correct normalized finding is therefore a `MEDIUM` acceptance isolation/evidence defect, not a `HIGH` proven production thumbnail-renderer defect.

## Frozen-source chain

### Fixture mutation without cleanup

`scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs`:

- seeds media through `seed-wiki-editorial-media.php`;
- calls `corrupt-files` for the seeded media;
- then calls `remove-files` for the same media;
- contains diagnostics hooks but no EditorialMedia reset before or after the test.

The next image-free test seeds another valid row and likewise performs no reset.

By contrast, `editorial-media-acceptance.spec.mjs` explicitly calls `editorialMediaFixture('reset')` before and after every test. The Wiki-specific spec does not.

### Persistence behavior

`scripts/acceptance/seed-browser-editorial-media.php`:

- `corrupt-files` overwrites both original and thumbnail objects but preserves the database row;
- `remove-files` deletes stored objects but preserves the database row;
- only the separate `reset` command deletes references, stored files and `editorial_media` rows.

`scripts/acceptance/seed-wiki-editorial-media.php` creates the row and files but provides no cleanup operation.

### Response behavior

`app/EditorialMedia/Application/WikiEditorialMediaFileResponse.php` validates:

- disk identity;
- extension and MIME metadata;
- path format;
- byte size and SHA-256 metadata;
- file existence;
- actual byte size and SHA-256.

Missing or corrupt objects throw `WikiEditorialMediaUnavailable`, a `RuntimeException`. The thumbnail controller does not convert that exception to a different response class.

The dedicated Editorial Media fallback test explicitly accepts HTTP 500 for the intentionally corrupt thumbnail and asserts that the UI renders an accessible fallback. This establishes that the historical status code was the exercised integrity-failure contract, not an unexplained response from a valid object.

## Exact execution order and ID accumulation

The preserved JUnit and Playwright reports show the same ordering in both historical runs.

### Portability phase

For each of Chromium, Firefox and WebKit:

1. `admin-wiki-administration.spec.mjs` runs;
2. `admin-wiki-editorial-media.spec.mjs` runs its corrupt/remove scenario;
3. the image-free Wiki media scenario runs.

The corrupt/remove scenario leaves stale odd-numbered rows. After portability, IDs `1`, `3` and `5` have missing stored objects while rows remain queryable.

### Responsive phase

The responsive report records:

1. desktop administration test;
2. desktop Wiki media tests;
3. tablet administration test;
4. tablet Wiki media tests;
5. mobile administration test;
6. mobile Wiki media tests.

This predicts the exact observed accumulation:

| Administration project | Stale corrupt/missing rows visible before the test | Recorded HTTP 500 count |
|---|---|---:|
| desktop | `1, 3, 5` | 9 |
| tablet | `1, 3, 5, 7` | 12 |
| mobile | `1, 3, 5, 7, 9` | 16 |

The same sequence and counts occur in both exact historical runs. The mobile total is 16 rather than a simple three-per-row total because the newest accumulated row is requested once more in that page lifecycle; this does not alter the identity of the stale rows.

## Impact

Proven impact:

- acceptance profiles are not isolated;
- a later Wiki scenario receives intentionally damaged rows created by unrelated prior scenarios;
- repeated expected integrity-failure HTTP 500 traffic contaminates diagnostics for otherwise valid Wiki lifecycle tests;
- the pollution can obscure unrelated failures and makes browser evidence order-dependent;
- it creates concurrent session-bearing thumbnail traffic around publication flows.

Not proven:

- valid production EditorialMedia thumbnails fail;
- the integrity-failure response itself violates a decided production contract;
- the leaked thumbnail requests caused the missing publication flash;
- the same historical pattern appears in a clean, isolated frozen-target Wiki run.

## Corrected finding

```yaml
id: OTERYN-AUDIT-P35-006
title: Wiki acceptance profiles leak intentionally damaged EditorialMedia rows into later tests
fact_state: PROVEN
severity: MEDIUM
confidence: HIGH
environment: REPO_MAIN plus CI_PROVEN historical execution
backend_status: integrity checks behave as designed for missing or corrupt objects
frontend_status: accessible preview fallback is rendered
integration_status: Wiki media fixture rows survive after their files are corrupted or removed and are consumed by later projects
state_coverage: exact historical cross-profile sequence proven twice; clean isolated frozen-target probe remains pending
impact: order-dependent acceptance evidence, repeated expected HTTP 500 diagnostics and possible interference with unrelated session-bearing lifecycle assertions
recommendation: reset Wiki EditorialMedia fixtures before and after each Wiki media test, then separately test missing/corrupt fallback with scoped assertions
shared_cause_with_flash_loss: NOT_PROVEN
```

## Validator implications

The independent validator must run two distinct cases:

1. **Clean isolated Wiki administration flow** — reset EditorialMedia before every sample and determine whether any thumbnail HTTP 500 or publication-flash loss occurs.
2. **Controlled polluted flow** — intentionally create one missing/corrupt row, prove that only that row returns the expected integrity-failure response, and verify whether concurrent requests affect flash persistence.

A clean run without 500 responses does not invalidate this finding; it confirms the isolation diagnosis. A valid-object 500 in a clean run would be a separate application defect and must be classified independently.
