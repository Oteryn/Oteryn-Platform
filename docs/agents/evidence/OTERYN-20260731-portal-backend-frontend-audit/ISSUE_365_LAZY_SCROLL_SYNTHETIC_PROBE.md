# Issue #365 responsive lazy-scroll synthetic probe

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Classification: `CONTROLLED_SYNTHETIC / DERIVED`; not direct execution of the Oteryn application and not a replacement for the exact frozen-target validator packet.

## Question

Can Playwright's actionability scroll for a far-down publication button create new native lazy-image work after the test has already observed an idle page, with a materially larger responsive gap on narrow layouts?

This question matters because the preserved original flow waits for `networkidle` and then starts the `Publish` click only 5–7 ms later. The Wiki form places a responsive media grid before the publication controls, and its thumbnails use native `loading="lazy"`.

## Corrected request boundary

The missing status is absent from the first rendered article-edit page after the publish redirect. Requests started only by that new page cannot be the primary reason the status was absent from its server-rendered HTML.

The viable request family is therefore narrower:

1. the old article-edit document is idle at its current scroll position;
2. Playwright begins the far-down publication action and scrolls the old document;
3. that scroll activates previously deferred old-document thumbnail work;
4. the publish POST writes the one-request `status` flash;
5. a queued old-document media request can acquire the serialized session before the redirect GET, save the session and age the flash;
6. the redirected document then renders durable publication state without the transient alert.

Steps 3–6 remain a runtime hypothesis until exact request and session-lock ordering is captured. This probe tests only whether step 3 is technically and responsively plausible.

## Environment and method

- Chromium `144.0.7559.96` on Debian;
- Playwright Python sync API;
- three samples per viewport and mode, 18 samples total;
- viewports: desktop `1440×900`, tablet `820×1180`, mobile `390×844`;
- 12 native `loading="lazy"`, `decoding="async"` images;
- responsive grid: three columns desktop, two tablet and one mobile;
- publication control below the media grid;
- initial 300 ms settle before action;
- direct mode: click the far-down publication control immediately after settle;
- control mode: explicitly scroll the publication control into view, settle another 300 ms, then click.

The container browser policy blocks network and file navigation, so the probe used `page.set_content()` and data-URI SVG images. Native browser lazy loading and Playwright actionability scrolling were exercised, but no Oteryn HTTP request, Laravel session or server lock was involved.

## Results

| Profile | Document height | Images loaded before direct click | New lazy-image loads after direct click | First completion after click | New loads after pre-scroll + settle |
|---|---:|---:|---:|---:|---:|
| desktop | 2685 px | 12/12 | 0 | n/a | 0 |
| tablet | 3729 px | 8/12 | 4 (`9–12`) | 16.1–17.2 ms | 0 |
| mobile | 6281 px | 3/12 | 4 (`9–12`) | 12.9–17.9 ms | 0 |

All three samples in every row produced the same counts and IDs.

The direct action moved the viewport from the top to:

- desktop: `scrollY=1785`;
- tablet: `scrollY=2549`;
- mobile: `scrollY=5437`.

Desktop had already loaded all 12 images before the action. Tablet and mobile had deferred lower-grid images and completed four new lazy loads immediately after the action-induced scroll. Explicit pre-scroll plus settle moved that work out of the click window and produced zero post-click lazy loads in all samples.

## Interpretation

### Proven by this synthetic probe

- a Playwright click on a far-down control can change the old document's viewport after an earlier settled state;
- responsive grid height materially changes how many native lazy images remain deferred before the action;
- tablet and mobile action-induced scroll can activate deterministic new lazy-image work while desktop has no remaining lazy work;
- an explicit pre-scroll and settle removes that post-click lazy-load work in this controlled layout.

### Not proven by this synthetic probe

- exact thumbnail request-start timestamps relative to the Oteryn publish POST;
- whether the preserved failures contained old-document thumbnail requests;
- session-lock acquisition order;
- whether valid, missing or corrupt media changes the race;
- whether any thumbnail request actually consumed `status` in attempts 3 or 4;
- exact frozen-target clean behavior.

The synthetic result therefore strengthens the **old-document lazy-request race family** but does not promote it to direct causal proof.

## Required exact differential probe

The mutable exact-target validator should add these paired mobile samples without committing the observer:

### Immediate-action sample

1. reset EditorialMedia and clear cache;
2. load the original Wiki administration flow;
3. wait for the existing pre-publication idle boundary;
4. click `Publish` normally;
5. record monotonic browser request-start, server request-entry, session-lock acquire/release, session-save and redirect-document timestamps.

### Pre-scroll control

Repeat the same sample, but before publication:

1. explicitly scroll the `Publish` control into view;
2. wait for media requests to settle;
3. then click `Publish`;
4. capture the same evidence.

For every media request capture its media ID, status, `Referer`, request/correlation ID and whether it originated before or after the publish navigation began. Log sanitized `_flash.new`, `_flash.old` and the presence of `status` at session load/save boundaries.

Run both modes for clean media and exactly one controlled missing/corrupt row. A difference between immediate action and pre-scroll control would support the action-scroll trigger; identical behavior would reject or weaken it.

## Verdict impact

The normalized findings remain **0 HIGH, 6 MEDIUM and 1 LOW**. `OTERYN-AUDIT-P35-005` remains `REPRODUCED_INTERMITTENT` and `NOT_PROVEN_REMEDIATED`. The audit verdict remains `VALIDATED_WITH_CORRECTIONS`, and the exact validator packet remains mandatory.