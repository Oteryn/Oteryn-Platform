# ADR 0012 — Public Wiki read, rendering and search boundary

## Status

Accepted — 2026-07-26

## Context

ADR 0010 established Platform-owned Wiki persistence, publication lifecycle, bilingual translations and restricted source Markdown without activating a public renderer or route. The first public slice must expose only effective published content, preserve the public localization policy, render editor-controlled Markdown safely and keep search replaceable without coupling controllers to database details.

## Decision

### 1. Public Wiki is a read-only module boundary

Anonymous controllers depend on `PublicWikiQuery`, `WikiSearch` and `WikiMarkdownRenderer` interfaces. Query and search implementations apply publication, locale, translation-freshness and category-visibility constraints before data reaches controllers or Blade templates.

This slice adds no administrator mutation, permission, upload, preview or cross-repository behavior.

### 2. Canonical routes use the shared public locale namespace

Canonical Wiki routes are:

```text
/{locale}/wiki
/{locale}/wiki/search
/{locale}/wiki/category/{slug}
/{locale}/wiki/{slug}
```

Supported locales remain exactly `en` and `pl`. Legacy non-localized routes remain deterministic English compatibility endpoints. Article and category `hreflang` links are emitted only when the equivalent target is public.

### 3. Polish freshness is evaluated against English source timestamps

English remains the explicit public editorial source. A Polish article or category translation is public only when its `updated_at` is not older than the matching English translation.

This uses the existing translation timestamps and adds no schema or lifecycle mutation. Missing or stale Polish content is excluded from home, category, article, equivalent-URL and search queries; English content is never substituted on a Polish URL.

### 4. CommonMark is rendered from source on every public article read

`league/commonmark` 2.8 is a direct dependency. The Wiki renderer uses a bounded parser configuration with:

- raw HTML stripped;
- unsafe CommonMark links disabled;
- explicit link output restricted to fragments, absolute internal paths and credential-free HTTPS URLs;
- images replaced by inert text placeholders until approved Wiki media integration exists;
- deterministic collision-safe heading identifiers and a generated table of contents;
- bounded nesting and delimiter parsing;
- responsive table wrappers.

Rendered HTML is not persisted or trusted as an input.

### 5. Initial search remains database-backed behind an interface

The initial implementation searches published translations with escaped literal wildcard handling, exact/prefix title preference, bounded query/page sizes, deterministic tie ordering and 12-result pagination.

Search is isolated by locale, applies the same translation-freshness boundary as public reads and is rate-limited by locale plus source IP. A later search engine may replace the implementation behind `WikiSearch` without changing controllers or publication rules.

### 6. Failure states are explicit

Missing, unpublished, hidden and stale resources return not found without revealing source content. Empty result sets remain successful truthful pages. Query dependency failures return a no-index 503 Wiki-unavailable page and are reported through the existing application exception boundary.

## Consequences

- Public navigation may expose Wiki because the delivered routes are complete rather than placeholders.
- Drafts, review content, archived content, future publication, missing translations and stale Polish translations cannot cross the public query/search boundary.
- Safe Markdown behavior is security-critical and covered by focused regression tests.
- Search relevance is intentionally bounded and portable; it does not claim language-specific full-text stemming.
- Wiki media remains disabled until a separate consumer integration adopts the approved EditorialMedia boundary.
- No Canary/login-server schema, credential, session or protocol changes occur.

## Rejected alternatives

### Trust persisted rendered HTML

Rejected. It would create a second security-sensitive source of truth and make renderer-policy upgrades unreliable.

### Allow all CommonMark links and remote images

Rejected. It would permit uncontrolled protocols, credential-bearing URLs and tracking content before an approved media policy exists.

### Fall back from Polish to English

Rejected. It would misrepresent translation availability and violate the public localization policy.

### Couple controllers directly to Eloquent or SQL

Rejected. Publication and locale constraints must remain centralized and testable, and the search engine must remain replaceable.

### Add a dedicated search service now

Rejected for the initial content volume. The database implementation is deterministic, bounded and sufficient behind an explicit interface.
