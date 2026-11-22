# Read-only production smoke

From `portal-rs/e2e`, use the existing Playwright dependencies and Chromium.

```sh
npm ci
npx playwright install chromium
npx playwright test --config live/playwright.live.config.mjs
```

The default target is `https://oteryn.com`. Set `LIVE_BASE_URL` to use another
HTTP(S) origin, or `LIVE_CHROMIUM` to select a system Chromium executable.
The four projects cover Polish and English at 1440px and 390px.

Run the suite with one worker and zero retries as configured. An anonymous
request gate serializes outbound GET and HEAD requests, spaces starts by at
least 350 ms, and shares a hard 2,000-request budget across all four projects.
Chromium has a closed outbound proxy and receives responses through CDP.
The gate checks every redirect hop it follows and excludes state-changing URLs,
credentialed requests, and origins outside the target, www, and API hosts.
Cookies are inspected from response headers and omitted from outbound requests.
The registration form is inspected without filling or submitting it.

Each locale starts at home and explicitly checks registration and the three
legal pages. The crawl renders up to three URLs per path pattern while checking
the status of discovered same-origin links, including links beyond that sample.
It checks markup-declared and browser-requested assets, browser errors, page
structure, mobile overflow, and the headers on home and the first available
non-legal inner page. The language check uses the document language, common UI
labels, and text heuristics for wrong-language copy, raw keys, and mojibake.
The axe check uses the existing `axe-core` dependency if available.

`LIVE SUMMARY` records the number of rendered pages and cumulative outbound
requests. `LIVE DEFECT` records hard findings with their source URLs.
`LIVE NOTE` records legal content status, a pending security.txt 404, and
unavailable or excluded coverage. The JSON report for each project is attached
to the Playwright result in the run's temporary directory.

While the Game feed is pending, a known public Game-data route with HTTP 503
is report-only when it serves HTML with the route's localized unavailable view
and both `Cache-Control: no-store` and the explicit `X-Oteryn-Game-Data: unavailable` signal. The check requires the matching
document language and the specific heading and refusal markers in the main
content. Unexpected error responses and failing assets remain hard findings.
