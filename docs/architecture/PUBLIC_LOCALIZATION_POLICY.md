# Public localization policy

## Supported locales

The public website supports exactly `en` and `pl`. `en` is the explicit default locale unless `PUBLIC_DEFAULT_LOCALE` is set to another supported value.

Canonical public URLs always include the locale as their first path segment:

- `/en/...`
- `/pl/...`

Authentication, account and administration URLs are not part of the public locale namespace.

## Negotiation and legacy compatibility

The legacy root URL `/` negotiates in this order:

1. the explicit `lang` query parameter;
2. the `oteryn_locale` cookie;
3. `Accept-Language`;
4. the configured default locale.

All other legacy non-localized public URLs remain deterministic English compatibility endpoints. They return the existing resource with `Content-Language: en` and identify the equivalent `/en/...` URL as canonical. They do not negotiate from browser preferences.

Selecting a language writes the locale cookie through the canonical public response. Explicit locale-prefixed URLs always win over cookie or browser language.

## Fallback policy

Static interface strings use Laravel's English fallback locale when a translation key is accidentally absent.

Editorial content never falls back between languages:

- the existing source record is the English publication;
- Polish content requires a separate explicit translation record;
- no translation record is generated automatically;
- no source text is copied into a translation field;
- no machine translation is performed;
- an English article, page, announcement or release note is never rendered as Polish.

A missing, draft, future-dated or stale Polish translation is excluded from Polish public queries. A direct Polish detail URL returns the localized unavailable/not-found state without exposing the English body.

## Translation states

Every Polish editorial translation has one of five derived states:

- `missing`: no translation record exists;
- `incomplete`: a record exists but one or more required translated fields are empty;
- `draft`: a complete record exists but has no effective publication timestamp;
- `published`: the translation is effective and reviewed against the current English source revision;
- `stale`: the English source was updated after the translation was reviewed.

Saving a translation records the current source `updated_at` value. A later source edit therefore makes the translation stale without deleting it. Republishing requires an editor to review and save the Polish translation again.

## Route equivalence and SEO

The language switcher links to an equivalent canonical route when it exists. Editorial detail pages do not offer a target locale when its translation is missing or stale. Event detail routes use the existing locale-specific event slugs.

Public pages emit:

- one canonical URL for the active locale;
- `hreflang` links only for available equivalents;
- `x-default` pointing to the English equivalent when available.

## Formatting

Public dates and numbers are formatted by the active locale:

- English uses English month names, `.` decimals and `,` grouping;
- Polish uses Polish month names, `,` decimals and non-breaking-space grouping.

Application storage and domain timestamps remain UTC. Localization changes presentation only.

## Domain boundaries

This foundation does not alter publication, artifact, calendar, wiki or runtime-data rules in Downloads, Events, Wiki or PublicGameData. It only adds routing, presentation and explicit translation availability at the public boundary.
