# Staging portal visual audit

Audit date 2026-11-23. Chromium rendered the real portal at `https://staging.oteryn.com`.
Source references use `origin/main` at `7847cf7de91957d4317e7693d31d049b8f1c5f8e`.
The anonymous crawl produced 210 full-page captures across English and Polish at
1440×900, 768×1024 and 390×844. The desktop Playwright projects also run the tablet slice.
Capture files live in ignored `test-results/`. The other actor inventories in
[pages.json](pages.json) record how far the audit could get and which routes still need evidence.

## Scenario results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Anonymous visual crawl | Fail, documented drift | Fail, documented drift | Fail, documented drift | Fail, documented drift |
| Sign-in input and refusal | Pass | Pass | Pass | Pass |
| News menu language switch | Pass | Pass | Pass | Pass |
| Registration validation and inbox handoff | Fail, HTTP 503 | Blocked, HTTP 429 | Blocked, HTTP 429 | Fail, HTTP 503 |
| Verified player and character | Fail at setup, HTTP 503 | Fail at setup, HTTP 503 | Fail at setup, HTTP 503 | Fail at setup, HTTP 503 |
| Staff recovery, menus and CMS | Blocked at shared setup | Blocked at shared setup | Fail at setup, HTTP 503 for both roles | Blocked at shared setup |

Staff setup ran once in en-1440 for both staff roles and would have unlocked both
languages and all widths. The English desktop player setup reached HTTP 503 on
the qualifying run and hit the shared rate limit on the focused rerun.

## Page inventory

| Route | Final HTTP | Discovery | State |
| --- | --- | --- | --- |
| `/static/thais/shop-{locale}.html` | 200 | menu | — |
| `/{locale}/` | 200 | home | — |
| `/{locale}/atlas/delivery` | 503 | menu | unavailable |
| `/{locale}/bazaar` | 503 | menu | unavailable |
| `/{locale}/bazaar/items` | 503 | template | unavailable |
| `/{locale}/characters` | 404 | template | unmounted or refused |
| `/{locale}/collection` | 200 | menu | redirect to /{locale}/login |
| `/{locale}/collection-pvp` | 200 | template | redirect to /{locale}/login |
| `/{locale}/events` | 200 | menu | — |
| `/{locale}/forgot-password` | 200 | template | — |
| `/{locale}/help` | 200 | menu | — |
| `/{locale}/league` | 503 | menu | unavailable |
| `/{locale}/legal/cookies` | 200 | template | — |
| `/{locale}/legal/privacy` | 200 | menu | — |
| `/{locale}/legal/terms` | 200 | menu | — |
| `/{locale}/login` | 200 | menu | — |
| `/{locale}/mfa/login` | 200 | template | — |
| `/{locale}/news` | 200 | menu | — |
| `/{locale}/puzzles` | 200 | menu | redirect to /{locale}/login |
| `/{locale}/register` | 200 | template | — |
| `/{locale}/register/check-inbox` | 200 | template | — |
| `/{locale}/rules` | 200 | menu | — |
| `/{locale}/search` | 200 | template | — |
| `/{locale}/server-information` | 200 | menu | — |
| `/{locale}/stats/rankings/anniversaries` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/month` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/name-changes` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/oldest` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/record` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/sold` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/transfers` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/week` | 404 | menu | unmounted or refused |
| `/{locale}/stats/rankings/yesterday` | 404 | menu | unmounted or refused |
| `/{locale}/support/report-a-bug` | 200 | menu | — |
| `/{locale}/today` | 200 | menu | — |
| `/{locale}/wiki` | 200 | menu | — |
| `/{locale}/wiki/places` | 503 | template | unavailable |
| `/{locale}/wiki/search` | 200 | template | — |

The route graph starts at each locale home and follows the header, footer, account and
in-page menus, with chapter/article discovery capped at eight links per page and a total
cap of 95 unique requests per actor, locale and viewport. It also opens explicit template
variants. The four crate-local template families and their route prerequisites are listed
in [pages.json](pages.json). The blocked actor rows are source candidates.

## Component styles

Representative values below are from the English 1440 slice and appear in order as font
size, weight, text colour, background, corner radius and padding. Distinct signatures
also include font family, border, line height, shadow, and outline. Fingerprints contain
all variants, the full shell box coordinates, content gutters, form-control heights,
and link hover / field focus styles.

| Component | Styles | Pages | Most frequent computed signature |
| --- | --- | --- | --- |
| header | 3 | 35 | `16px, 500, rgb(246, 244, 235), rgba(12, 25, 29, 0.76), 0px, 0px` |
| nav | 3 | 35 | `16px, 500, rgb(246, 244, 235), rgba(0, 0, 0, 0), 0px, 0px` |
| footer | 3 | 35 | `16px, 500, rgb(237, 243, 237), rgba(12, 25, 29, 0.76), 0px, 0px` |
| headerLink | 10 | 35 | `20px, 600, rgb(246, 244, 235), rgba(0, 0, 0, 0), 0px, 0px` |
| footerLink | 5 | 35 | `31.68px, 800, rgb(237, 243, 237), rgba(0, 0, 0, 0), 0px, 0px` |
| logo | 3 | 35 | `31.68px, 800, rgb(237, 243, 237), rgba(0, 0, 0, 0), 0px, 0px` |
| active | 5 | 35 | `14px, 650, rgb(246, 221, 161), rgba(0, 0, 0, 0), 0px, 0px` |
| main | 4 | 35 | `16px, 500, rgb(244, 242, 234), rgba(0, 0, 0, 0), 6px, 0px` |
| h1 | 2 | 35 | `56px, 600, rgb(244, 242, 234), rgba(0, 0, 0, 0), 0px, 0px` |
| h2 | 3 | 7 | `40.32px, 600, rgb(244, 242, 234), rgba(0, 0, 0, 0), 0px, 0px` |
| h3 | 1 | 1 | `20px, 700, rgba(0, 0, 0, 0), rgb(246, 221, 161), 0px, 0px` |
| body | 14 | 35 | `16px, 500, rgb(244, 242, 234), rgb(16, 25, 29), 0px, 0px` |
| small | 1 | 1 | `14px, 500, rgb(185, 198, 199), rgba(0, 0, 0, 0), 0px, 0px` |
| link | 7 | 19 | `16px, 500, rgb(246, 221, 161), rgba(0, 0, 0, 0), 0px, 0px` |
| primary | 1 | 4 | `15px, 750, rgb(246, 221, 161), rgba(16, 31, 31, 0.91), 5px, 8.8px 16.8px` |
| secondary | 1 | 15 | `15px, 750, rgb(246, 221, 161), rgba(16, 31, 31, 0.91), 5px, 8.8px 16.8px` |
| danger | 0 | — | No example in this slice |
| button | 1 | 1 | `16px, 500, rgb(255, 255, 255), rgb(21, 32, 39), 0px, 0px` |
| input | 1 | 7 | `16px, 500, rgb(244, 242, 234), rgb(25, 30, 25), 4px, 8px 12px` |
| select | 1 | 10 | `16px, 500, rgb(244, 242, 234), rgb(25, 30, 25), 4px, 8px 12px` |
| textarea | 0 | — | No example in this slice |
| table | 0 | — | No example in this slice |
| tableHeader | 0 | — | No example in this slice |
| tableOdd | 0 | — | No example in this slice |
| tableEven | 0 | — | No example in this slice |
| tableCell | 0 | — | No example in this slice |
| alert | 1 | 1 | `15px, 500, rgb(255, 157, 146), rgba(0, 0, 0, 0), 0px, 0px` |
| status | 1 | 12 | `16px, 500, rgb(244, 242, 234), rgba(0, 0, 0, 0), 0px, 0px` |
| card | 1 | 3 | `16px, 500, rgb(244, 242, 234), rgba(17, 31, 36, 0.78), 6px, 24px` |
| badge | 0 | — | No example in this slice |
| pagination | 0 | — | No example in this slice |
| breadcrumb | 0 | — | No example in this slice |
| empty | 0 | — | No example in this slice |

Header and footer x, width and height are compared to the modal page with values
rounded to CSS pixels. Footer y depends on content length. Component comparisons use
exact computed style values. Favicon responded with an SVG image. Chromium hides
the visited-history paint from script, so the fingerprints record author `:visited`
rules from accessible stylesheets. The home and all captured route colours stayed the
same when the browser switched from light to dark preference.

## Palette values seen on one or two pages

This view lists observed outliers per actor, locale and viewport. A rare value may be
intentional. Browser defaults, responsive sizing and artwork account for some values.

<details>
<summary>Computed palette outliers (116 distinct values)</summary>

| Property group | Value | Page and locale/width |
| --- | --- | --- |
| colors | `rgb(105, 80, 28)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(12, 23, 26)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(158, 172, 171)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(192, 206, 202)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(208, 219, 220)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(21, 32, 39)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(216, 199, 165)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(22, 34, 38)` | `/static/thais/shop-{locale}.html` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(223, 235, 235)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(23, 35, 40)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(245, 235, 216)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(255, 157, 146)` | `/{locale}/register` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(37, 28, 13)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(56, 69, 73)` | `/static/thais/shop-{locale}.html` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768), `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgb(94, 101, 87)` | `/static/thais/shop-{locale}.html` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgba(115, 136, 140, 0.37)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgba(12, 25, 29, 0.69)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgba(16, 32, 39, 0.52)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgba(16, 35, 38, 0.396)` | `/{locale}/` (en 390, en 768, pl 390, pl 768) |
| colors | `rgba(207, 192, 156, 0.35)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgba(239, 245, 233, 0.15)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `rgba(240, 225, 198, 0.22)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| colors | `url("#home-gilt-stroke") rgb(246, 221, 161)` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| families | `ui-monospace, "Cascadia Code", "Segoe UI Mono", monospace` | `/{locale}/legal/cookies` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| radii | `32px` | `/static/thais/shop-{locale}.html` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| radii | `3px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| radii | `50%` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| shadows | `rgb(229, 197, 141) 0px 0px 0px 1px inset, rgba(19, 18, 15, 0.8) 0px 1px 3px 0px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| shadows | `rgba(3, 13, 17, 0.12) 0px 4px 16px 0px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| sizes | `12px` | `/{locale}/` (en 390, pl 390) |
| sizes | `13.28px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| sizes | `18.4px` | `/static/thais/shop-{locale}.html` (en 1440, en 768, pl 1440, pl 768) |
| sizes | `18px` | `/static/thais/shop-{locale}.html` (en 390, pl 390), `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| sizes | `19px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| sizes | `23.2px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| sizes | `25px` | `/static/thais/shop-{locale}.html` (en 768, pl 768), `/{locale}/` (en 768, pl 768) |
| sizes | `28px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| sizes | `32px` | `/static/thais/shop-{locale}.html` (en 390, en 768, pl 390, pl 768) |
| sizes | `34.56px` | `/{locale}/` (en 1440, pl 1440) |
| sizes | `45px` | `/{locale}/` (en 390, pl 390) |
| sizes | `48px` | `/{locale}/` (en 768, pl 768) |
| sizes | `64.8px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `-187.2px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `-65px` | `/{locale}/` (en 390, en 768, pl 390, pl 768) |
| spacing | `-84px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `-96px` | `/{locale}/` (en 390, pl 390) |
| spacing | `-99.84px` | `/{locale}/` (en 768, pl 768) |
| spacing | `10.4px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| spacing | `100px` | `/{locale}/` (en 768, pl 768) |
| spacing | `11.2px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| spacing | `121.6px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `124.344px` | `/{locale}/forgot-password` (pl 768) |
| spacing | `130px` | `/{locale}/` (en 768, pl 768) |
| spacing | `13px` | `/{locale}/` (en 390, pl 390) |
| spacing | `140.766px` | `/{locale}/forgot-password` (en 768) |
| spacing | `145px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `14px` | `/static/thais/shop-{locale}.html` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768), `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| spacing | `15.2031px` | `/{locale}/register/check-inbox` (en 1440, pl 1440), `/{locale}/wiki/places` (en 1440, pl 1440) |
| spacing | `15.36px` | `/{locale}/` (en 768, pl 768) |
| spacing | `157.492px` | `/{locale}/` (en 390, pl 390) |
| spacing | `17px` | `/{locale}/` (en 1440, en 768, pl 1440, pl 768) |
| spacing | `199.234px` | `/{locale}/league` (en 768, pl 768) |
| spacing | `19px` | `/{locale}/` (en 1440, en 768, pl 1440, pl 768) |
| spacing | `205.125px` | `/{locale}/help` (pl 768), `/{locale}/support/report-a-bug` (pl 768) |
| spacing | `212.734px` | `/{locale}/wiki/places` (pl 768) |
| spacing | `214px` | `/{locale}/league` (en 390, pl 390) |
| spacing | `221.547px` | `/{locale}/help` (en 768), `/{locale}/support/report-a-bug` (en 768) |
| spacing | `229.156px` | `/{locale}/wiki/places` (en 768) |
| spacing | `22px` | `/static/thais/shop-{locale}.html` (en 1440, en 768, pl 1440, pl 768) |
| spacing | `237.492px` | `/{locale}/` (en 390, pl 390) |
| spacing | `239.125px` | `/{locale}/register/check-inbox` (pl 768) |
| spacing | `23px` | `/{locale}/` (en 1440, en 768, pl 1440, pl 768) |
| spacing | `25.9844px` | `/{locale}/login` (en 768), `/{locale}/mfa/login` (en 768) |
| spacing | `255.281px` | `/{locale}/bazaar/items` (pl 768) |
| spacing | `255.547px` | `/{locale}/register/check-inbox` (en 768) |
| spacing | `25px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `271.125px` | `/{locale}/bazaar` (pl 768) |
| spacing | `281.516px` | `/{locale}/events` (pl 768), `/{locale}/news` (pl 768) |
| spacing | `287.547px` | `/{locale}/bazaar` (en 768) |
| spacing | `28px` | `/{locale}/` (en 768, pl 768) |
| spacing | `297.516px` | `/{locale}/atlas/delivery` (pl 768) |
| spacing | `297.938px` | `/{locale}/events` (en 768), `/{locale}/news` (en 768) |
| spacing | `299.281px` | `/{locale}/league` (pl 1440) |
| spacing | `305.906px` | `/{locale}/rules` (pl 768), `/{locale}/server-information` (pl 768) |
| spacing | `306.016px` | `/{locale}/bazaar/items` (en 768) |
| spacing | `308.484px` | `/{locale}/league` (en 1440) |
| spacing | `30px` | `/static/thais/shop-{locale}.html` (en 1440, pl 1440), `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| spacing | `313.938px` | `/{locale}/atlas/delivery` (en 768) |
| spacing | `32.3438px` | `/{locale}/search` (pl 768) |
| spacing | `322.328px` | `/{locale}/rules` (en 768), `/{locale}/server-information` (en 768) |
| spacing | `32px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `34.3438px` | `/{locale}/wiki` (pl 768), `/{locale}/wiki/search` (pl 768) |
| spacing | `38.3906px` | `/{locale}/bazaar/items` (en 1440) |
| spacing | `38px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `390.484px` | `/{locale}/league` (pl 1440) |
| spacing | `421.406px` | `/{locale}/league` (en 1440) |
| spacing | `45px` | `/{locale}/` (en 390, pl 390) |
| spacing | `46.08px` | `/static/thais/shop-{locale}.html` (en 768, pl 768) |
| spacing | `47.2031px` | `/{locale}/bazaar` (en 1440, pl 1440) |
| spacing | `48.7656px` | `/{locale}/search` (en 768) |
| spacing | `50.7656px` | `/{locale}/wiki` (en 768), `/{locale}/wiki/search` (en 768) |
| spacing | `57.5938px` | `/{locale}/events` (en 1440, pl 1440), `/{locale}/news` (en 1440, pl 1440) |
| spacing | `58px` | `/static/thais/shop-{locale}.html` (en 390, en 768, pl 390, pl 768) |
| spacing | `5px` | `/{locale}/` (en 768, pl 768) |
| spacing | `61.25px` | `/{locale}/` (en 768, pl 768) |
| spacing | `62px` | `/{locale}/` (en 768, pl 768) |
| spacing | `64px` | `/{locale}/league` (en 1440, pl 1440) |
| spacing | `6px` | `/static/thais/shop-{locale}.html` (en 390, pl 390), `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| spacing | `73.5938px` | `/{locale}/atlas/delivery` (en 1440, pl 1440) |
| spacing | `7px` | `/{locale}/` (en 390, pl 390) |
| spacing | `81.9844px` | `/{locale}/rules` (en 1440, pl 1440), `/{locale}/server-information` (en 1440, pl 1440) |
| spacing | `9.5625px` | `/{locale}/login` (pl 768), `/{locale}/mfa/login` (pl 768) |
| spacing | `9.6px` | `/{locale}/` (en 1440, en 390, en 768, pl 1440, pl 390, pl 768) |
| spacing | `93.6px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `95.5156px` | `/{locale}/` (en 1440, pl 1440) |
| spacing | `96px` | `/static/thais/shop-{locale}.html` (en 1440, pl 1440) |

</details>

## Layout and translation

Counts below are distinct observations per page and component signature, with duplicates
collapsed for each page. Text and overlap tolerance is one CSS pixel. Minimum tap
target is 40×40px with 0.5px measurement tolerance. Inline links are included.
Aspect-ratio tolerance is 2% where the image is sized without preserving its aspect.
Colour contrast checks use actual composited solid CSS and the WCAG AA thresholds of
4.5:1 and 3:1 for large text. The last column counts unscored samples with artwork,
gradients, group opacity, or a colour value the numeric check cannot resolve.

| Locale | Width | Offscreen | Clip | Overlap | Tap size | Contrast | Image / icon | Unscored text |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| en | 1440 | 0 | 0 | 0 | 0 | 0 | 0 | 334 |
| en | 768 | 0 | 0 | 0 | 0 | 0 | 0 | 334 |
| en | 390 | 0 | 0 | 0 | 215 | 0 | 0 | 334 |
| pl | 1440 | 0 | 0 | 0 | 0 | 0 | 0 | 334 |
| pl | 768 | 0 | 0 | 0 | 0 | 0 | 0 | 334 |
| pl | 390 | 0 | 0 | 0 | 178 | 0 | 0 | 334 |

The language comparison pairs the same normalized path at the same width. A changed
wrap count can be normal for a longer translation if its containing box still fits.
The runnable news-menu scenario also switches language via the actual footer, opens
the menu and checks wrapping and geometry.

| Width | Route | Polish / English difference |
| --- | --- | --- |
| 390 | `/{locale}/league` | a 2 / 1 |


## Titles, headings and active links

| Locale | Route | Browser title | Active labels | Visible h1 | Mismatch |
| --- | --- | --- | --- | --- | --- |
| en | `/{locale}/` | `Home · Oteryn` | (logo) | News | heading label |
| en | `/{locale}/login` | `Sign in · Oteryn` | — | Sign in | active nav |
| en | `/{locale}/help` | `Player support` | — | Player support | title, active nav |
| en | `/{locale}/rules` | `This page has not been published yet.` | Game rules | This page has not been published yet. | title, heading label |
| en | `/{locale}/server-information` | `This page has not been published yet.` | Server information | This page has not been published yet. | title, heading label |
| en | `/{locale}/legal/cookies` | `Cookie Policy` | — | Cookie Policy | title |
| en | `/{locale}/wiki/places` | `Hunting, bestiary and boss places` | — | Hunting, bestiary and boss places | title |
| en | `/{locale}/stats/rankings/yesterday` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/week` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/month` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/record` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/oldest` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/anniversaries` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/name-changes` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/transfers` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/stats/rankings/sold` | `Growth rankings` | — | Growth rankings | title, active nav |
| en | `/{locale}/bazaar/items` | `Item trading is temporarily unavailable. Deposits and reserved funds remain protected.` | — | Item trading is temporarily unavailable. Deposits and reserved funds remain protected. | title |
| en | `/{locale}/atlas/delivery` | `Quest delivery and item sources` | Delivery routes | Quest delivery and item sources | title, heading label |
| en | `/{locale}/league` | `World events and guild seasons` | Leagues | World events and guild seasons | title, heading label |
| en | `/{locale}/bazaar` | `Character auctions unavailable` | Character bazaar | Character auctions unavailable | title, heading label |
| en | `/{locale}/news` | `News` | News | News | title |
| en | `/{locale}/events` | `Events` | Events | Events | title |
| en | `/{locale}/today` | `Today` | Today in Oteryn | Today | title, heading label |
| en | `/{locale}/support/report-a-bug` | `Report a bug` | Report a bug | Report a bug | title |
| en | `/{locale}/legal/terms` | `Terms of Service` | Terms of service | Terms of Service | title |
| en | `/{locale}/legal/privacy` | `Privacy Policy` | Privacy policy | Privacy Policy | title |
| pl | `/{locale}/` | `Strona główna · Oteryn` | (logo) | Aktualności | heading label |
| pl | `/{locale}/login` | `Zaloguj się · Oteryn` | — | Zaloguj się | active nav |
| pl | `/{locale}/help` | `Pomoc dla graczy` | — | Pomoc dla graczy | title, active nav |
| pl | `/{locale}/rules` | `Ta strona nie została jeszcze opublikowana.` | Zasady gry | Ta strona nie została jeszcze opublikowana. | title, heading label |
| pl | `/{locale}/server-information` | `Ta strona nie została jeszcze opublikowana.` | Informacje o serwerze | Ta strona nie została jeszcze opublikowana. | title, heading label |
| pl | `/{locale}/legal/cookies` | `Polityka plików cookie` | — | Polityka plików cookie | title |
| pl | `/{locale}/wiki/places` | `Miejsca polowań, bestiariusza i bossów` | — | Miejsca polowań, bestiariusza i bossów | title |
| pl | `/{locale}/stats/rankings/yesterday` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/week` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/month` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/record` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/oldest` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/anniversaries` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/name-changes` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/transfers` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/stats/rankings/sold` | `Rankingi przyrostu` | — | Rankingi przyrostu | title, active nav |
| pl | `/{locale}/bazaar/items` | `Handel przedmiotami jest tymczasowo niedostępny. Depozyty i zarezerwowane środki pozostają zabezpieczone.` | — | Handel przedmiotami jest tymczasowo niedostępny. Depozyty i zarezerwowane środki pozostają zabezpieczone. | title |
| pl | `/{locale}/atlas/delivery` | `Mapa dostaw zadań i gdzie wypada` | Trasy dostaw | Mapa dostaw zadań i gdzie wypada | title, heading label |
| pl | `/{locale}/league` | `Wydarzenia świata i sezony gildii` | Rozgrywki | Wydarzenia świata i sezony gildii | title, heading label |
| pl | `/{locale}/bazaar` | `Aukcje postaci są niedostępne` | Bazar postaci | Aukcje postaci są niedostępne | title, heading label |
| pl | `/{locale}/news` | `Aktualności` | Aktualności | Aktualności | title |
| pl | `/{locale}/events` | `Wydarzenia` | Wydarzenia | Wydarzenia | title |
| pl | `/{locale}/today` | `Dziś` | Dzisiaj w Oterynie | Dziś | title, heading label |
| pl | `/{locale}/support/report-a-bug` | `Zgłoś błąd` | Zgłoś błąd | Zgłoś błąd | title |
| pl | `/{locale}/legal/terms` | `Regulamin Oteryn` | Regulamin | Regulamin Oteryn | title, heading label |
| pl | `/{locale}/legal/privacy` | `Polityka prywatności` | Polityka prywatności | Polityka prywatności | title |


## Entity consistency and form states

| Field or transition | Browser surfaces | Evidence |
| --- | --- | --- |
| Account name, email, role | Account overview, dashboard, account pages, staff player view | Blocked at player registration and staff password recovery. Mailpit recovered after the deployment. |
| Character name, level, creation date | Account overview, character page, public projection and highscores | Blocked before authenticated character creation. No character was submitted. |
| News title, date and body | Editor revision, public archive/detail, homepage teaser, both locales | Public archive and home rendered an empty state. Staff publication and cross-surface comparison were blocked. Source archive/detail render title/body without a publication date. |
| Sign-in refusal | Form heading, alert, submit area, second tab | Both locales put the localized `.op-error` alert between the h1 and form. The second tab keeps its own clean view. |
| Registration validation and handoff | Form field error, inbox confirmation | Native email format validation ran. Registration POST returned HTTP 429 on rate-limited attempts and HTTP 503 on valid English and Polish attempts after browser verification returned HTTP 200. No confirmed handoff. The Polish desktop mismatch rendered inline validation. |
| Staff reset | Public recovery form, delivered mail, changed-password confirmation | Both test roles received their reset mail. The new-password POST returned HTTP 503 for both. |
| Destructive confirmations | Signed-in player and staff UI | No evidence after the authentication blockers. |
| Dates, times and numbers | Legal text, visible data times/cells, public feed | No populated staging feed or ranking to compare. Legal edition identifiers display ISO-style dates. The fingerprint lists recognizable rendered date patterns where present. |

## Rendered terminology

| Concept | PL | EN | Conflict |
| --- | --- | --- | --- |
| Sign in | Zaloguj się | Log in / Sign in | English shop/header and sign-in form use different verbs for the same destination. |
| Create a portal account | Utwórz konto / Zarejestruj się | Create an account / Register | Registration heading and entry/submit labels switch concepts in both locales. |
| Email address | E-mail / Adres e-mail | E-mail / Email address / E-mail verification | English registration, MFA sign-in and confirmation alternate hyphenation. |
| Today | Dziś / Dzisiaj w Oterynie | Today / Today in Oteryn | Footer destination and heading use different labels. |
| Portal rules | Regulamin / Regulamin Oteryn | Terms of service / Terms of Service | Polish footer and heading differ in scope. English varies only in capitalization. |
| Delivery routes | Trasy dostaw / Mapa dostaw zadań i gdzie wypada | Delivery routes / Quest delivery and item sources | Menu and heading describe the same route with different terms. |
| Leagues | Rozgrywki / Wydarzenia świata i sezony gildii | Leagues / World events and guild seasons | Menu label changes scope in the heading. |
| Commerce availability | Aukcje postaci są niedostępne / Handel przedmiotami jest tymczasowo niedostępny / Sklep jest obecnie zamknięty | Character auctions unavailable / Item trading is temporarily unavailable / The shop is currently closed | Equivalent unavailable states use different heading and notice patterns. |

## Screenshot review

- Home, hero and H1 at every width. News occupies the sole h1 while the home title
  identifies the page as Home. The hero, profession cards and bestiary are visually
  coherent and main artwork scales without a stretched appearance.
- The 390px footers on interior pages pack short link boxes into a long list and
  reduce the language buttons below the design target. The Polish legal row wraps
  sooner. The mobile Shop header uses a different second-row text navigation.
- The 768px Shop changes alignment and navigation density compared with 768px News.
  The interior header uses a visible Menu control, while Shop places four inline links
  in its own bar. The Shop closure is clearly explained in the body.
- Wiki and Wiki Search put a separate Oteryn Wiki Portal Polish/English link inside
  the content panel in addition to the footer language switch. The rankings use a
  plain list of equally styled tabs and still show filters when no ranking is found.
- Character and item Bazaar notices use different hierarchy. The item refusal promotes
  both the unavailable explanation and reserved-funds sentence to the H1. The long
  heading takes up several lines at 390px. The character auction page keeps the
  headline short and puts the explanation below it. League displays its unavailable
  notice and then two empty sections, leaving headings with no content.
- The news archive, events, legal editions, help, support, search, reset and account
  entry/confirmation pages retain the same dark shell and panel treatment. Password
  bars in captures are screenshot masks. The empty state copy is visible on News,
  Events, Wiki and Today.

## Complete measurements outside tolerance

These rows list measured drift and failures. [FINDINGS.md](FINDINGS.md) groups
confirmed product problems and gives their sources. Repeated CSS measurements can
appear on multiple routes because the underlying markup is shared.

<details>
<summary>Every measured page and component outside tolerance</summary>

| Route | Component | Check | Scope | Expected | Observed | Screenshot |
| --- | --- | --- | --- | --- | --- | --- |
| `/static/thais/shop-{locale}.html` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | h1 | style drift | en 390, en 768, pl 390, pl 768 | `[{"fontFamily":"Alegreya, Georgia, serif","fontSize":"31.2px","fontWeight":"600","lineHeight":"34.32px"}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"fontFamily":"Alegreya, Georgia, serif","fontSize":"32px","fontWeight":"600","lineHeight":"35.2px"}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-768-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | header | style drift | en 1440, pl 1440 | `[{"backgroundColor":"rgba(12, 25, 29, 0.76)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,1440,84]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"backgroundColor":"rgb(22, 34, 38)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,1440,84]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | header | style drift | en 390 | `[{"backgroundColor":"rgba(12, 25, 29, 0.76)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,390,65]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"backgroundColor":"rgb(22, 34, 38)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,390,110]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | header | style drift | pl 390 | `[{"backgroundColor":"rgba(12, 25, 29, 0.76)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,390,65]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"backgroundColor":"rgb(22, 34, 38)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,390,144]]}]` | [static_thais_shop-pl_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/static_thais_shop-pl_html.png) |
| `/static/thais/shop-{locale}.html` | header | style drift | en 768, pl 768 | `[{"backgroundColor":"rgba(12, 25, 29, 0.76)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,768,65]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"backgroundColor":"rgb(22, 34, 38)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[0,0,768,65]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-768-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | header / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `36.8×49.6px` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | header / a.shop-language | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `61.8×30.8px` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | header / a.shop-language | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `70.4×30.8px` | [static_thais_shop-pl_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/static_thais_shop-pl_html.png) |
| `/static/thais/shop-{locale}.html` | logo | style drift | en 390, pl 390 | `[{"boxes":[[22,122,40]]},{"boxes":[[24,122,40]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"boxes":[[24,122,40]]},{"boxes":[[24,122,40]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | logo | style drift | en 768, pl 768 | `[{"boxes":[[24,146,48]]},{"boxes":[[40,146,48]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"boxes":[[24,146,48]]},{"boxes":[[46,146,48]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-768-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | logo | style drift | en 1440, pl 1440 | `[{"boxes":[[56,146,48]]},{"boxes":[[86,146,48]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"boxes":[[86,146,48]]},{"boxes":[[86,146,48]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `35.5×44px` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | nav | style drift | en 1440 | `[{"backgroundColor":"rgba(0, 0, 0, 0)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[246,20,1138,44]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"backgroundColor":"rgba(0, 0, 0, 0)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[1060,15,294,53]]}]` | [static_thais_shop-en_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/static_thais_shop-en_html.png) |
| `/static/thais/shop-{locale}.html` | nav | style drift | pl 1440 | `[{"backgroundColor":"rgba(0, 0, 0, 0)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[246,20,1138,44]]}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"backgroundColor":"rgba(0, 0, 0, 0)","color":"rgb(246, 244, 235)","fontFamily":"Manrope, system-ui, sans-serif","boxes":[[902,15,451,53]]}]` | [static_thais_shop-pl_html.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-1440/anonymous-pl-1440-light/static_thais_shop-pl_html.png) |
| `/{locale}/` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | h1 | style drift | en 390, en 768, pl 390, pl 768 | `[{"fontFamily":"Alegreya, Georgia, serif","fontSize":"31.2px","fontWeight":"600","lineHeight":"34.32px"}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"fontFamily":"Alegreya, Georgia, serif","fontSize":"23.2px","fontWeight":"600","lineHeight":"27.84px"}]` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-768-light/home.png) |
| `/{locale}/` | h1 | style drift | en 1440, pl 1440 | `[{"fontFamily":"Alegreya, Georgia, serif","fontSize":"56px","fontWeight":"600","lineHeight":"61.6px"}] (modal style, shell boxes rounded to a CSS pixel)` | `[{"fontFamily":"Alegreya, Georgia, serif","fontSize":"23.2px","fontWeight":"600","lineHeight":"27.84px"}]` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/home.png) |
| `/{locale}/` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135×26px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/home.png) |
| `/{locale}/` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `170.4×26px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | main / a.hero-news-all | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `138.1×31.9px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/home.png) |
| `/{locale}/` | main / a.hero-news-all | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `50.5×31.9px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | main / a.step-rules | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `138×21px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | main / a.step-rules | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×21px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/home.png) |
| `/{locale}/` | main / a.step-rules | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `80×21px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/home.png) |
| `/{locale}/` | main / a.step-rules | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `82.4×21px` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/home.png) |
| `/{locale}/` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [home.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/home.png) |
| `/{locale}/atlas/delivery` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [atlas_delivery.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/atlas_delivery.png) |
| `/{locale}/atlas/delivery` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [atlas_delivery.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/atlas_delivery.png) |
| `/{locale}/atlas/delivery` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [atlas_delivery.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/atlas_delivery.png) |
| `/{locale}/atlas/delivery` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [atlas_delivery.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/atlas_delivery.png) |
| `/{locale}/atlas/delivery` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [atlas_delivery.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/atlas_delivery.png) |
| `/{locale}/bazaar` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [bazaar.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/bazaar.png) |
| `/{locale}/bazaar` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [bazaar.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/bazaar.png) |
| `/{locale}/bazaar` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [bazaar.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/bazaar.png) |
| `/{locale}/bazaar` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [bazaar.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/bazaar.png) |
| `/{locale}/bazaar` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [bazaar.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/bazaar.png) |
| `/{locale}/bazaar/items` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [bazaar_items.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/bazaar_items.png) |
| `/{locale}/bazaar/items` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [bazaar_items.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/bazaar_items.png) |
| `/{locale}/bazaar/items` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [bazaar_items.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/bazaar_items.png) |
| `/{locale}/bazaar/items` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [bazaar_items.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/bazaar_items.png) |
| `/{locale}/characters` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [characters.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/characters.png) |
| `/{locale}/characters` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [characters.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/characters.png) |
| `/{locale}/characters` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [characters.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/characters.png) |
| `/{locale}/events` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [events.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/events.png) |
| `/{locale}/events` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [events.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/events.png) |
| `/{locale}/events` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [events.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/events.png) |
| `/{locale}/events` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [events.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/events.png) |
| `/{locale}/forgot-password` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [forgot-password.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/forgot-password.png) |
| `/{locale}/forgot-password` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [forgot-password.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/forgot-password.png) |
| `/{locale}/forgot-password` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [forgot-password.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/forgot-password.png) |
| `/{locale}/forgot-password` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `320×26.4px` | [forgot-password.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/forgot-password.png) |
| `/{locale}/help` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/help.png) |
| `/{locale}/help` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/help.png) |
| `/{locale}/help` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/help.png) |
| `/{locale}/help` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/help.png) |
| `/{locale}/help` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `175.5×22px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/help.png) |
| `/{locale}/help` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `192.7×22px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/help.png) |
| `/{locale}/help` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `215.2×22px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/help.png) |
| `/{locale}/help` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `238.3×22px` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/help.png) |
| `/{locale}/help` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [help.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/help.png) |
| `/{locale}/league` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [league.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/league.png) |
| `/{locale}/league` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [league.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/league.png) |
| `/{locale}/league` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [league.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/league.png) |
| `/{locale}/league` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [league.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/league.png) |
| `/{locale}/league` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [league.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/league.png) |
| `/{locale}/legal/cookies` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `101.8×22px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `136.4×22px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `154.1×22px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_cookies.png) |
| `/{locale}/legal/cookies` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `155.3×22px` | [legal_cookies.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/legal_cookies.png) |
| `/{locale}/legal/privacy` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `100.2×22px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `136.4×22px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `154.1×22px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_privacy.png) |
| `/{locale}/legal/privacy` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `163.1×22px` | [legal_privacy.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/legal_privacy.png) |
| `/{locale}/legal/terms` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/legal_terms.png) |
| `/{locale}/legal/terms` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `101.8×22px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `136.4×22px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `154.1×22px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `154.5×22px` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/legal_terms.png) |
| `/{locale}/legal/terms` | navigation / main h1 | heading label | pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [legal_terms.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-1440/anonymous-pl-1440-light/legal_terms.png) |
| `/{locale}/login` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/login.png) |
| `/{locale}/login` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/login.png) |
| `/{locale}/login` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/login.png) |
| `/{locale}/login` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/login.png) |
| `/{locale}/mfa/login` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [mfa_login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/mfa_login.png) |
| `/{locale}/mfa/login` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [mfa_login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/mfa_login.png) |
| `/{locale}/mfa/login` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [mfa_login.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/mfa_login.png) |
| `/{locale}/news` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [news.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/news.png) |
| `/{locale}/news` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [news.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/news.png) |
| `/{locale}/news` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [news.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/news.png) |
| `/{locale}/news` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [news.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/news.png) |
| `/{locale}/register` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [register.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register.png) |
| `/{locale}/register` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [register.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register.png) |
| `/{locale}/register` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [register.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register.png) |
| `/{locale}/register` | main form / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `320×26.4px` | [register.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register.png) |
| `/{locale}/register/check-inbox` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [register_check-inbox.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register_check-inbox.png) |
| `/{locale}/register/check-inbox` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [register_check-inbox.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register_check-inbox.png) |
| `/{locale}/register/check-inbox` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [register_check-inbox.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register_check-inbox.png) |
| `/{locale}/register/check-inbox` | main / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `320×26.4px` | [register_check-inbox.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/register_check-inbox.png) |
| `/{locale}/rules` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [rules.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/rules.png) |
| `/{locale}/rules` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [rules.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/rules.png) |
| `/{locale}/rules` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [rules.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/rules.png) |
| `/{locale}/rules` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [rules.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/rules.png) |
| `/{locale}/rules` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [rules.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/rules.png) |
| `/{locale}/search` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/search.png) |
| `/{locale}/search` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/search.png) |
| `/{locale}/search` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/search.png) |
| `/{locale}/server-information` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [server-information.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/server-information.png) |
| `/{locale}/server-information` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [server-information.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/server-information.png) |
| `/{locale}/server-information` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [server-information.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/server-information.png) |
| `/{locale}/server-information` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [server-information.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/server-information.png) |
| `/{locale}/server-information` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [server-information.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/server-information.png) |
| `/{locale}/stats/rankings/anniversaries` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/anniversaries` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_anniversaries.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_anniversaries.png) |
| `/{locale}/stats/rankings/month` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/month` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_month.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_month.png) |
| `/{locale}/stats/rankings/name-changes` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/name-changes` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_name-changes.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_name-changes.png) |
| `/{locale}/stats/rankings/oldest` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/oldest` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_oldest.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_oldest.png) |
| `/{locale}/stats/rankings/record` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/record` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_record.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_record.png) |
| `/{locale}/stats/rankings/sold` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/sold` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_sold.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_sold.png) |
| `/{locale}/stats/rankings/transfers` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/transfers` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_transfers.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_transfers.png) |
| `/{locale}/stats/rankings/week` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/yesterday` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/week` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_week.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_week.png) |
| `/{locale}/stats/rankings/yesterday` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `111.4×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `115.1×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `117.6×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `134.2×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `135.8×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `143.2×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `148.4×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `148×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `149.1×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `150.5×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.3×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `161.4×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `162.3×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.1×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `173.5×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `184.1×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `235.6×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | main nav / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `97.7×22px` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | menu | menu destination | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an advertised destination leads to a usable page or a documented unavailable state` | `HTTP 404 from /:locale/stats/rankings/week` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_yesterday.png) |
| `/{locale}/stats/rankings/yesterday` | navigation | active nav | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `an exact-destination link has aria-current=page` | `no matching link marked active` | [stats_rankings_yesterday.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/stats_rankings_yesterday.png) |
| `/{locale}/support/report-a-bug` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `175.5×22px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `192.7×22px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | main / a | tap target | pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `215.2×22px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/support_report-a-bug.png) |
| `/{locale}/support/report-a-bug` | main / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `238.3×22px` | [support_report-a-bug.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/support_report-a-bug.png) |
| `/{locale}/today` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [today.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/today.png) |
| `/{locale}/today` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [today.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/today.png) |
| `/{locale}/today` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [today.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/today.png) |
| `/{locale}/today` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [today.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/today.png) |
| `/{locale}/today` | navigation / main h1 | heading label | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `heading matches an exact active link label` | `labels differ` | [today.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/today.png) |
| `/{locale}/wiki` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [wiki.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki.png) |
| `/{locale}/wiki` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [wiki.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki.png) |
| `/{locale}/wiki` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [wiki.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki.png) |
| `/{locale}/wiki` | header / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `44×22px` | [wiki.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki.png) |
| `/{locale}/wiki` | header / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `54.6×22px` | [wiki.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki.png) |
| `/{locale}/wiki` | header / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `85.7×22px` | [wiki.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki.png) |
| `/{locale}/wiki/places` | document title | title | en 1440, en 390, en 768, pl 1440, pl 390, pl 768 | `<Page> · Oteryn` | `suffix missing` | [wiki_places.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/wiki_places.png) |
| `/{locale}/wiki/places` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [wiki_places.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_places.png) |
| `/{locale}/wiki/places` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [wiki_places.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_places.png) |
| `/{locale}/wiki/places` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [wiki_places.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_places.png) |
| `/{locale}/wiki/search` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `33.7×44px` | [wiki_search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_search.png) |
| `/{locale}/wiki/search` | footer / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `36×44px` | [wiki_search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_search.png) |
| `/{locale}/wiki/search` | footer / a | tap target | en 390 | `at least 40×40px (0.5px rounding tolerance)` | `38.8×44px` | [wiki_search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_search.png) |
| `/{locale}/wiki/search` | header / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `44×22px` | [wiki_search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_search.png) |
| `/{locale}/wiki/search` | header / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `54.6×22px` | [wiki_search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_search.png) |
| `/{locale}/wiki/search` | header / a | tap target | en 390, pl 390 | `at least 40×40px (0.5px rounding tolerance)` | `85.7×22px` | [wiki_search.png](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/wiki_search.png) |

</details>

## Reproduce

Run from `portal-rs/e2e`.

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/visual/playwright.config.mjs
```

Set `STAGING_ORIGIN` to audit another host. The qualifying run used
`STAGING_PROXY="$HTTPS_PROXY"` and `STAGING_IGNORE_HTTPS_ERRORS=1` for the
sandbox proxy’s Chromium redirect certificate. Traces and diagnostic page snapshots
are disabled. Set the two Cloudflare Access variables for staging in the environment.
Only requests to the exact staging host use them.
