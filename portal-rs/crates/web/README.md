# Public web surface

Add one route function per domain module and append its `.merge(...)` call to
`module_router()`. That router uses `AppState` and receives the same body limit,
error rendering and security policy as the built-in endpoints. Keep internal
and administrative listeners separate from this public router.

The `/` entry redirects to `/pl/` or `/en/`. Supported `portal_language` cookie
values take precedence over weighted `Accept-Language`; unsupported input falls
back to Polish. An explicit language prefix controls the displayed page.
`/language/pl` and `/language/en` set a bounded locale cookie and redirect to the
localized home page. These GET links change only the display preference.

Handlers can extract `Extension<Language>` and call `Language::text()` for shared
Fluent messages. Add matching keys to `../ui/i18n/pl.ftl` and `../ui/i18n/en.ftl`. The plain
Askama layout is `layout.html`; styling and components belong to T39.

Use `Extension<CspNonce>` if a page needs an inline script and place that value
in its `nonce` attribute. Never use `unsafe-inline`, event-handler attributes or
inline style attributes. Nonces are fresh for every response; HTML is not cached.

`router()` enables HSTS. Local HTTP embedders can explicitly use
`router_with_options(state, static_dir, WebOptions { hsts: false })`. This option
does not weaken CSP or the other response headers.

Static filenames containing a dot- or hyphen-delimited hexadecimal digest of at
least eight characters receive one-year immutable caching. Other successful
static responses cache for five minutes with revalidation. Use a new digest
whenever immutable file content changes. Content types come from `ServeDir`.

Database tests use `PORTAL_TEST_DATABASE_URL`. The readiness integration test
prints a skip notice when this variable is absent. It performs only a readiness
query and does not require migrations or modify database state.
