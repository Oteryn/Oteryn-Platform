# Staging visual findings

Findings are ranked by visibility to a player. Each entry identifies its staging actor and scope.
`/{locale}` means both English and Polish unless the scope says otherwise.
The desktop Playwright projects include their 768px audit. Screenshot paths refer
to the ignored `test-results/` directory relative to this file.

## PRODUCT DEFECT 1 — Valid registration hits an unavailable page

As a new anonymous player, open `/{locale}/register`, enter a fresh email and
matching generated strong passwords, accept both policies and submit. The
handler should redirect with HTTP 303 to the inbox confirmation. The browser
verification request returned HTTP 200, but the standard form POST returned
HTTP 503. It rendered a localized registration-unavailable screen and never
reached confirmation. Reproduced in both languages at 1440×900 and 390×844.

Sources [register.rs:151](../../../../crates/web/src/routes/register.rs#L151)
and [abuse/http.rs:41](../../../../crates/identity/src/abuse/http.rs#L41).
[English phone screenshot](test-results/registration-evidence/actors-verified-player-and-character-views-stay-consistent-en-390/form-states/registration-unavailable.png).
[Polish desktop screenshot](test-results/registration-evidence/actors-verified-player-and-character-views-stay-consistent-pl-1440/form-states/registration-unavailable.png).

## PRODUCT DEFECT 2 — Small phone tap targets

Open `/{locale}/news` at 390×844 and scroll to the footer. A footer section link
is 33px high and the PL/EN language controls are 36px wide. The design
audit requires at least 40×40px. Small targets also affect in-content links,
consent controls and the Shop header. The portal token gives interactive targets
a 44px minimum. Representative values and every affected component are in the
[layout evidence](REPORT.md#layout-and-translation).

Sources [tokens.css:81](../../../../static/css/tokens.css#L81),
[shell.css:367](../../../../static/thais/shell.css#L367) and
[shell.css:393](../../../../static/thais/shell.css#L393).
[English phone screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-en-390/anonymous-en-390-light/news.png).

## PRODUCT DEFECT 3 — Shop changes the shared shell

Open Shop from the header at 1440, 768 and 390. Expect the shared masthead, same
navigation controls and recurring wordmark position. The visible closed Shop
fallback uses its own inline navigation, opaque `rgb(22, 34, 38)` header instead of
`rgba(12, 25, 29, 0.76)`, and a wrapped second row on phones. Its footer also
changes height where the alternate links wrap. Shop closure itself is explained.

Sources [layout.html:18](../../../../templates/layout.html#L18),
[shop-en.html:15](../../../../static/thais/shop-en.html#L15) and
[shop.css:43](../../../../static/thais/shop.css#L43).
[Polish phone Shop screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/static_thais_shop-pl_html.png).

## PRODUCT DEFECT 4 — Ranking tabs point to unavailable views

Open any `/{locale}/stats/rankings/…` view at 1440, 768 or 390 and follow a tab.
An advertised tab should open a ranking or show a clearly disabled state. All
nine rendered tabs are styled as usable links, return HTTP 404 with “Ranking not
found” or its Polish equivalent, and offer working-looking filters. The browser
never shows which tab is current. Ranking data being disabled is the staging
condition. The active links and filter presentation are the visual problem.

Sources [rankings/index.html:4](../../../../templates/stats/rankings/index.html#L4),
[rankings_http.rs:219](../../../../crates/stats/src/rankings_http.rs#L219).
[English tablet rankings screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-768-light/stats_rankings_yesterday.png).

## PRODUCT DEFECT 5 — Browser titles lose the Oteryn suffix

Visit `/{locale}/news`, events, help, Today, legal pages and the other routes in
the title table at 1440, 768 or 390. The shared template expects
`<Page> · Oteryn`. The restored portal shell contains bare titles such as
“News”, “Events” and “Player support”. Browser tabs and history lose the same
branding that Home, sign-in and registration retain.

Sources [layout.html:7](../../../../templates/layout.html#L7),
[news.rs:53](../../../../crates/content/src/news.rs#L53),
[editorial.html:1](../../../../templates/pages/editorial.html#L1).
[English desktop news screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/news.png).

## PRODUCT DEFECT 6 — Sign-in and Help never indicate the current destination

Open `/{locale}/login` via the masthead and `/{locale}/help` via the footer.
Each shared navigation link should identify the current destination like the
News link does. Neither destination has a highlighted current item. This
reproduces at 1440, 768 and 390, with the mobile menu open for sign-in.

Sources [thais-header.html:127](../../../../templates/components/thais-header.html#L127) and
[thais-footer.html:45](../../../../templates/components/thais-footer.html#L45).
[English desktop login screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/login.png).

## PRODUCT DEFECT 7 — Home’s main heading names the news section

Open the home route at each width. Expect its main heading to identify the same
destination as the home logo and document title. The sole H1 reads “News” or
“Aktualności”. The desktop H1 is 23.2px where the usual page H1 is 56px. The title
still reads “Home · Oteryn” or “Strona główna · Oteryn”.

Sources [home.html:8](../../../../templates/pages/home.html#L8),
[home.html:37](../../../../templates/pages/home.html#L37) and
[interior.css:133](../../../../static/thais/interior.css#L133).
[English desktop home screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/home.png).

## PRODUCT DEFECT 8 — Destination headings change the navigation label

Use the Today, rules and server information links at any width. Expect the
selected destination title to remain recognizable in its H1. Today in Oteryn
becomes Today, and Dzisiaj w Oterynie becomes Dziś. On the rules and server
information routes the heading is a generic unpublished notice that never names
the requested page. In Polish, the footer label “Regulamin” becomes “Regulamin
Oteryn” in the legal Terms page. The Atlas Delivery and League links likewise
change scope between menu and H1. The report lists every differing pair.

Sources [thais-footer.html:35](../../../../templates/components/thais-footer.html#L35),
[editorial.html:1](../../../../templates/pages/editorial.html#L1) and
[pages.rs:155](../../../../crates/content/src/pages.rs#L155).
[Polish desktop Today screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-1440/anonymous-pl-1440-light/today.png).

## PRODUCT DEFECT 9 — Disabled pages use different notice hierarchy

Compare `/{locale}/bazaar`, `/{locale}/bazaar/items` and Shop at all three
widths. Expect a short destination heading and a consistent unavailable notice
below. The item route uses the whole two-sentence notice as its H1, while the
character bazaar has a short headline and separate explanation. Shop also uses
a distinct notice component. At 390px the Polish item headline occupies several
lines before the footer. League also shows two bare section headings after its
unavailable notice with neither results nor standings beneath them.

Sources [items/web.rs:78](../../../../crates/bazaar/src/items/web.rs#L78),
[characters/http.rs:124](../../../../crates/bazaar/src/characters/http.rs#L124) and
[shop.css:19](../../../../static/thais/shop.css#L19) and
[league/web.rs:23](../../../../crates/minigames/src/league/web.rs#L23).
[Polish phone Bazaar items screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-pl-390/anonymous-pl-390-light/bazaar_items.png).

## PRODUCT DEFECT 10 — Entry forms switch terminology

Follow Log in from the English header and shop, and then follow Register.
Expect one term per action. The English form uses Sign in where the masthead and
shop say Log in. Registration switches between Create an account and Register,
and between Utwórz konto and Zarejestruj się. The English form flow also alternates
“E-mail” and “Email address” across registration, sign-in and verification.
Seen in both desktop and phone screenshots.

Sources [ui/i18n/en.ftl:87](../../../../crates/ui/i18n/en.ftl#L87),
[ui/i18n/en.ftl:269](../../../../crates/ui/i18n/en.ftl#L269),
[mfa/en.ftl:3](../../../../templates/account/mfa/en.ftl#L3) and
[ui/i18n/pl.ftl:87](../../../../crates/ui/i18n/pl.ftl#L87).
[English desktop registration screenshot](test-results/public-anonymous-page-inventory-and-visual-consistency-en-1440/anonymous-en-1440-light/register.png).

## PRODUCT DEFECT 11 — A delivered password-reset link cannot complete recovery

As the content editor or moderator at 1440×900 in English, request a password reset, open the
delivered link, and submit a new strong password with matching confirmation.
A valid completed reset should redirect to sign-in. The new-password POST
returned HTTP 503 and did not advance to sign-in. Mailpit had recovered by
this attempt. Browser evidence identifies the failed submission. The specific
unavailable recovery dependency remains unverified.

Source [reset_http.rs:330](../../../../crates/identity/src/reset_http.rs#L330).
[Content editor screenshot](test-results/recovery-evidence/actors-staff-menus-and-bilingual-editorial-publication-en-1440/staff-recovery/content-editor-reset-unavailable.png).
[Moderator screenshot](test-results/recovery-evidence/actors-staff-menus-and-bilingual-editorial-publication-en-1440/staff-recovery/moderator-reset-unavailable.png).

## Staging limitations

- During the first staging build the Mailpit list returned HTTP 404. A later
  deployment briefly returned HTTP 502 from readiness and then restored Mailpit
  HTTP 200. The recovered endpoint delivered reset messages to both staff actors.
- Shared-egress HTTP 429 limited registration and password recovery even after
  backoff. Native invalid-email validation still ran. The HTTP 429 responses are
  treated as staging noise. The HTTP 503s above occurred on attempts that got past
  the rate limit.
- Player verification, character creation, staff menus, destructive confirmations
  and the bilingual CMS entity comparison remained blocked at account setup.
- The staged Shop displays its closed notice. The character and item Bazaar and
  Wiki Places, Atlas Delivery and League render explicit unavailable states.
  The gated character search route rendered page-not-found. The public Collection
  and Puzzles entry points led to sign-in on the recovered deployment.
- An empty editorial feed gave the archive and homepage no existing news entity
  to compare. Rules and server information are unpublished on this deployment.

## Test bugs corrected

- The initial sign-in assertion used the ordinary-login strings. The installed MFA
  router rewrites `/{locale}/login` and renders the MFA refusal for rejected
  credentials. Both configured variants now assert their exact copy.
  [mfa_http.rs:143](../../../../crates/identity/src/mfa_http.rs#L143).
- Chromium exposes layout boxes for descendants of closed disclosure elements.
  The geometry checks now exclude controls inside closed disclosure panels and
  explicitly render the opened menu for language switching.
- Registration evidence originally sampled the previous document during a POST
  navigation. The test now waits for the new main frame and document before
  collecting the response text and screenshot. Shared egress limits can also
  affect the browser verification request, which has its own status.
- The first mobile language switch left the absolute-positioned menu expanded
  over the footer. The test now closes the menu using its own button before
  clicking the footer, and saves each language’s open-menu screenshot separately.

- Chromium’s native redirect requests needed the runner’s explicit proxy TLS
  override. Without it the redirected browser navigation hit
  `ERR_CERT_AUTHORITY_INVALID` even though the initial routed request succeeded.
  The qualifying run used the sandbox proxy and its TLS override.
