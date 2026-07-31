# Issue #365 historical artifact review

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Evidence scope: historical CI only; this file does not claim frozen-target reproduction.

## Reviewed artifacts

| Run | Exact tested SHA | Job | Artifact | Locally verified ZIP SHA-256 |
|---|---|---:|---:|---|
| `30562698853` | `35f39b48233b186502cbdcc05aec7ffc40e78fc7` | `90939481510` | `8767657461` | `8af4dedd1e213108a2599df303f45de7bf22caf603c180f7607ad5d8395a85c6` |
| `30578806660` | `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb` | `90993603962` | `8773887288` | `4a514e4a53d427599f07e0a22ad7cd918a154187e6ddd837e707ece2c14e96f2` |

Both artifacts identify:

- profile: `critical`;
- browser projects: Chromium, Firefox and WebKit portability plus responsive desktop/tablet/mobile;
- responsive viewports: 1440×1000, 820×1180 and 390×844;
- global and bounded profile retries: zero;
- runtime: real Laravel HTTP;
- dependencies: isolated MariaDB Platform and Canary schemas, Redis ACL and MailHog.

## Publication result

The same test failed in both artifacts:

`admin-wiki-administration.spec.mjs >> @wiki-admin trusted editor creates, previews and publishes bilingual Wiki content`

Only the `responsive-mobile` project failed. Desktop and tablet completed the test.

The failed assertion expected an accessible `role="status"` element containing `Wiki article published.` within 10 seconds. No status element was found.

The captured accessibility tree on the redirected page independently proves durable success:

- article status: `Published`;
- version: `3`;
- lifecycle action: `Unpublish to draft` present;
- public publication POST had already succeeded.

Therefore the historical symptom is loss of transient success feedback after durable publication, not publication failure.

## Thumbnail HTTP 500 pattern

The `browser-diagnostics` attachment records every response with status `>=500`. Both runs produced the same deterministic per-viewport pattern during the Wiki administration scenario:

| Viewport project | Test outcome | Total thumbnail HTTP 500 responses | Unique failing media IDs |
|---|---|---:|---|
| `responsive-desktop` | PASS | 9 | `1, 3, 5` |
| `responsive-tablet` | PASS | 12 | `1, 3, 5, 7` |
| `responsive-mobile` | FAIL on flash assertion | 16 | `1, 3, 5, 7, 9` |

All recorded server errors were GET requests to:

`/admin/wiki/media/{id}/thumbnail`

The same page rendered `Preview unavailable` fallbacks for affected approved-media cards. The 500 responses therefore affected a real administrator media-picker integration even when the overall test continued.

The identical 9/12/16 pattern across two separate exact heads materially raises confidence that the historical thumbnail symptom was deterministic for the accumulated seeded media set, rather than a single random network failure.

## Additional separate console defect

Every desktop, tablet and mobile execution in both artifacts also recorded two browser console errors caused by this HTML pattern value:

`[a-z0-9]+([._-][a-z0-9]+)*`

Chromium reported it as an invalid regular expression under the `v` flag because of the character class. The errors occurred on the Wiki category-create and article-create pages.

This is separate from both preserved Issue #365 symptoms. The current audit does not assign it a severity or remediation task because:

- it was not part of the original Issue #365 scope;
- the historical forms remained operable in the captured executions;
- frozen-target reproduction and current browser impact were not executed.

A validator should classify it explicitly if it reproduces on the frozen target; it must not be silently merged into the flash or thumbnail cause.

## Causality boundary

The artifacts prove temporal coexistence only:

- session-bearing thumbnail requests were active around the Wiki lifecycle flow;
- thumbnail endpoints returned repeated HTTP 500 responses;
- mobile lost the transient publication flash while durable publication succeeded.

They do not prove:

- that thumbnail requests discarded the flash;
- that request ordering caused either symptom;
- that the symptoms share one backend cause;
- that either symptom reproduces on `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`.

Current frozen-target classification remains `UNKNOWN` / `NOT_RUN` until the focused validator execution captures sanitized request, application and server evidence.
