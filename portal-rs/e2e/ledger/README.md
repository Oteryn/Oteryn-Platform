# Portal E2E coverage ledger

From the repository root, use the disposable PostgreSQL setup in the [browser qualification README](../README.md). With the local administrator URL available as `PORTAL_E2E_ADMIN_DATABASE_URL`, run:

```sh
CARGO_BUILD_JOBS=2 portal-rs/e2e/ledger/llvm-cov.sh
```

This entry point rebuilds the route and form inventories, runs all four Playwright projects against the same instrumented portal, and refreshes the request, form and LLVM reports. Pass `--gate` to also fail on every unhit request or form row. Playwright failures retain their usual nonzero exit status. Commit the regenerated inventories and aggregate reports when refreshing the baseline.

Raw Playwright results, traces, per-worker JSONL and LLVM profiles stay in the private run directory printed by the command. The JSONL files are isolated per run and separate from the Playwright output directory that gets cleared at startup.

The recorder installs in Playwright workers before context creation. Browser context events include popups and pages opened by specs. Playwright's pinned in-process transport exposes API request events for both standalone and browser-associated contexts, including individual redirect hops. Check these hooks when upgrading Playwright. Each request is associated with the active Playwright test, including its setup and teardown hooks.

The recorder matches the portal origin against `inventory.json`, preferring literal segments over parameters and catchalls. Paths absent from inventory and requests to other origins have a null route pattern. Requests with no HTTP response have a null status and are excluded from coverage. The inventory has two GET patterns with the same path shape for Bazaar items. The resolver uses inventory order for that tie.

The recorder uses the harness's disposable database connection to resolve the session cookie sent on each request. Authenticated accounts are represented by sorted role keys, with `player` for accounts without a staff role. Requests without an authenticated session are `anonymous`. Only inventory patterns, template-defined button names and values, actor labels, status codes and Playwright metadata are written.

Form rows group template locations by action pattern, method, and submitter name and value. An unnamed submitter gets its own row. The extractor scans HTML files in template directories, expands template conditionals and uses the owning renderers' paths for shared templates and implicit actions. Some form destinations are missing from the provisional route inventory. Those forms still have their own rows. The JSON identifies their mapping, when available, and includes each template location.

Form evidence is based on query or form-encoded fields sent to the matching action. It includes deliberate API form probes. An unnamed submitter is inferred from declared form fields. An ordinary navigation with a matching form query can produce the same evidence as a browser submission. Dynamic ranking pagination buttons are identified by the submitted page relative to the referring page.

Reports list observed actor and status combinations and their projects. Success means 2xx or 3xx. Refusal means 400, 401, 403, 404, or 422. Row IDs use a deterministic number derived from the row's identity and survive report reruns and insertion of unrelated rows. The gate writes both reports and returns nonzero if any route or form row has no hit, listing each missing row.

## Recorded full matrix

The complete instrumented run measured source `e114f50f1ada66ffd4592e0122f56826bf398032`. The request, form, and Rust coverage reports below use evidence from that run.

| Project | Passed | Failed | Timed out | Skipped |
| --- | ---: | ---: | ---: | ---: |
| pl-1440 | 496 | 0 | 0 | 0 |
| pl-390 | 496 | 0 | 0 | 0 |
| en-1440 | 494 | 1 | 1 | 0 |
| en-390 | 494 | 1 | 1 | 0 |

Both English locale sweeps reached the Polish password-reset page through the unprefixed forgot-password alias. The recovery-key journey timed out in `en-1440`, and the public link and accessibility sweep timed out in `en-390`. The run had zero runner errors and zero retries. [Rust coverage](LLVMCOV.md)

The preceding portal qualification recorded passing results for both journeys that timed out. Comparing 1,740 cases with the same project, spec and title found no other status changes. The three Polish calculator checks that failed on skip-link contrast in the request-recording lane passed in the merged matrix. The earlier LLVM lane's partial run failed staff MFA bootstrap, which also blocked dependent route probes. Those probes completed in the merged run. Merge diagnostics found stale Polish analytics assertions, missing test-only password-range data, a search translation tied to the wrong revision, and expired shared staff sessions. The affected cases passed in the full matrix.

## Review fixes and retests

The review reruns used source `dfb9926d1400b1327455fdbffb91e52cd79910c3`. Staff analytics now honors the saved language, the unprefixed forgot-password page redirects to the preferred locale, and the two long browser journeys have enough time to finish on an instrumented build. The scoped Playwright run includes the GET locale inventory, registration and password reset, recovery keys, public links and accessibility, and staff analytics filters, route access and language preference.

| Project | Passed | Failed | Timed out | Skipped |
| --- | ---: | ---: | ---: | ---: |
| pl-1440 | 18 | 0 | 0 | 0 |
| en-1440 | 18 | 0 | 0 | 0 |
| en-390 | 18 | 0 | 0 | 0 |

All four failures from the full matrix passed in these reruns. The review reran the affected cases in `pl-1440` and `en-390`, plus `en-1440` to reproduce the desktop English recovery journey.

Rust passed 656 cases across Admin (231), Community (207), Content (33), Stats (61), UI (10), Web (71), and Wiki (43). The Web count includes its opt-in Chromium test. The portal binary's password-reset redirect integration test also passed, as did formatting.

## Remaining evidence gaps

The ledger contains **545 rows**, with **485 hit** and **60 unhit**. The unhit set contains **45 forms without submission evidence** and **15 route-method rows**. The coverage gate exits 1 and lists each unhit row. [Request and form ledger](LEDGER.md)

| Reason | Unhit rows |
| --- | --- |
| Console mutation controls require a live Admin Agent advertising the operation and owning the target. | `F-admin-2051149960876`, `F-admin-38702896297808`, `F-admin-138082345336189`, `F-admin-253054411857215`, `R-admin-1919642460343`, `R-admin-122795670899350`, `R-admin-136010976080442`, `R-admin-260508168334342` |
| The configured staff fixture did not expose the live Agent player search POST form. | `F-admin-17456964450888` |
| The owned bestiary filter requires current Game bestiary progress. The configured source reports unavailable. | `F-atlas-161391314177223` |
| The qualified Bazaar runtime serves the account listing flow. The compatibility item creation and action forms are unmounted. | `F-bazaar-23275926065244`, `F-bazaar-46767873991968`, `F-bazaar-86148115874390`, `F-bazaar-140015055205774`, `F-bazaar-160340630540621`, `F-bazaar-187660078956952`, `F-bazaar-221617554685096`, `F-bazaar-231195288605257`, `F-bazaar-250676990346223`, `R-bazaar-13638167717685` |
| The listing filter requires a signed snapshot containing an item list. The pinned fixture omits items. | `F-bazaar-249147474695525` |
| The recorder maps this duplicate GET path shape to the live listing pattern with a different inventory row ID. | `R-bazaar-137999873732047` |
| The public filter is withheld until the relevant deaths, roster, guild or ranking projection and watermark are available. | `F-community-3881337848774`, `F-community-4730707544746`, `F-community-16287300610837`, `F-community-71546943467822`, `F-community-188835548187730`, `F-community-193593469157960`, `F-community-215566307321900`, `F-community-219222581560817`, `F-community-274165327503467` |
| The browser registration proof requests the unprefixed challenge endpoint. The localized POST alias has no separate probe. | `R-identity-59549485195707` |
| Collection deck controls require an accepted owned collection. Battle actions also require an active owned battle. | `F-minigames-24012547260821`, `F-minigames-114652082160743`, `F-minigames-177564890723493`, `F-minigames-187973382217026`, `F-minigames-218934052584363`, `F-minigames-280573004026262`, `R-minigames-115252755875764` |
| Season-pass claims and purchase controls require an admitted season and authoritative progress. | `F-minigames-58824648762686`, `F-minigames-267968195854043` |
| Native coin checkout requires an authenticated account with an existing Game-authorized order. | `F-payments-249659384691764` |
| The ranking direction buttons require multiple pages of Game ranks. | `F-stats-216992143813701`, `F-stats-222324429744041` |
| The rename and deletion form requires character-service lifecycle evidence beyond the LCFA ownership fixture. | `F-web-33906076394532`, `R-web-98846564089840` |
| The browser exercises consent reads. The origin and mutation checks are in the Web crate tests. | `R-web-133929700218240` |
| The browser CSRF probe uses the unprefixed badges POST. The localized POST alias has no separate probe. | `R-web-170115755742800` |
| The verification mail flow submits on the localized route. The unprefixed POST alias has no separate probe. | `R-web-251033525087013` |
| The Wiki entity, search and pagination filters need an admitted catalogue. The pinned candidate is deliberately unsupported. | `F-wiki-5428367139875`, `F-wiki-22119884537424`, `F-wiki-58594829099106`, `F-wiki-154492065254748`, `F-wiki-172496912900389`, `F-wiki-250551961713734` |
| World inspection and mutation require a selectable WorldStore version. The local catalogue remains inactive. | `F-world-86048466149919`, `F-world-89263600590152`, `R-world-71870839689412` |
| The browser commerce journeys leave the cancellation action unused. Its authorization and mutation are exercised by the Admin crate tests. | `R-admin-228096123067150` |
| The browser ticket journeys never submit the mark-read action. The Support crate has a focused mark-read test. | `R-support-96776198767987` |
