# ADR 0014: Wiki integration with EditorialMedia

- Status: Accepted
- Date: 2026-07-26

## Context

The public and administrator Wiki surfaces already use restricted CommonMark rendering, published-only locale-aware reads, short-lived signed preview, exact RBAC, confirmed MFA, optimistic locking and append-only revisions. The separate EditorialMedia module already owns normalized JPEG, PNG and WebP processing, immutable private storage, integrity metadata, bounded consumer references and reference-safe deletion.

The first consumer integration must let trusted Wiki editors reuse existing approved images without exposing the private disk, granting upload/delete authority implicitly, enabling remote images, leaking draft content or weakening publication freshness rules. The integration also needs deterministic deletion protection while current article translations reference media.

## Decision

### Canonical Markdown target

Wiki article Markdown uses standard image syntax with one exact local target form:

```markdown
![Localized contextual alternative text](wiki-media:123)
```

The target grammar is exactly `wiki-media:<positive-decimal-editorial-media-id>`.

- no whitespace, sign, leading zero, query, fragment, path, percent encoding or additional scheme component is accepted;
- remote URLs, relative paths, data URLs and every other image target are rejected by administrator validation and remain inert in the renderer as a defence-in-depth fallback;
- extraction and rendering must share one parser/target value object so validation, reference synchronization and output cannot interpret the same source differently;
- image nodes inside fenced or inline code are not references because extraction uses the CommonMark document tree rather than raw regular-expression scanning.

No database field stores a public path. The numeric identifier names an existing EditorialMedia record only.

### Alternative text

The Markdown alternative text is authoritative for Wiki rendering because it is contextual and localized with the article translation.

- it is required after plain-text normalization;
- it is bounded to 500 Unicode characters;
- raw HTML is not accepted as alternative text;
- the EditorialMedia `alt_text` value is an insertion-time default for the editor selector only and is not silently substituted during rendering;
- decorative empty-alt images are outside this initial scope.

This preserves localized accessibility without adding media-translation schema in this slice.

### Reference identity and synchronization

Current Wiki translations, not immutable historical revisions, own EditorialMedia references.

For each distinct media identifier used by one current translation, the reference slot is:

```text
consumer    = wiki
consumer_id = translation:<wiki_article_translation_id>
usage       = body.<editorial_media_id>
```

Repeated uses of the same media object in one translation create one reference row. Different translations create independent rows.

Create, update and revision restore must:

1. parse and validate every Wiki media token;
2. lock the Wiki article and enforce the existing optimistic version check before reference mutation;
3. verify every referenced EditorialMedia row exists and remains on the dedicated private disk;
4. save the translation/revision and reconcile the exact desired reference set within the same outer database transaction;
5. release removed current references only after the new desired set is valid.

A stale write, malformed token, missing media object or storage-boundary mismatch rolls back both content and reference changes. Existing article audit events remain authoritative; the derived reference set does not record article text or alternative text in audit metadata.

Historical revisions do not retain deletion-blocking references. Restoring a revision revalidates its current media identifiers and fails atomically with a bounded validation error if an object no longer exists. It never recreates or substitutes deleted media silently.

### Editor discovery and authority

A user with the existing exact `wiki.articles.manage` permission may use a Wiki-specific read-only selector for existing EditorialMedia metadata and integrity-checked thumbnails.

This capability does not grant `media.manage` and therefore does not permit upload, replacement or deletion. Upload and deletion continue to require the existing EditorialMedia administration routes, confirmed MFA and exact `media.manage` permission.

The selector exposes only bounded fields needed for selection: identifier, normalized thumbnail/content URL, dimensions, verified MIME and the insertion-default alternative text. It never exposes storage paths, digests, uploader identifiers or original bytes.

### Rendering context

Wiki rendering receives an explicit context rather than relying on ambient request state.

Public article rendering provides the effective article translation identifier and locale. Signed administrator preview provides the current translation identifier, article identifier, locale and a short-lived signed-media URL factory. A context-free renderer continues to neutralize image nodes.

The renderer emits responsive `<img>` output only for canonical tokens that resolve to a reference owned by the active rendering context. It escapes the contextual Markdown alternative text and does not accept arbitrary attributes, dimensions, classes or URLs from Markdown.

### Public delivery authorization

The localized public route is:

```text
GET /{locale}/wiki/media/{editorialMedia}
```

The route is declared before the generic Wiki article slug route and accepts a positive integer identifier only. It does not expose a storage path or original filename.

Before reading bytes, public delivery must prove that the requested media has a current `wiki` reference owned by at least one effective published translation in the requested locale. The authorization query reuses the same effective-publication rules as public Wiki reads:

- article status is `published`;
- `published_at` is present and not in the future;
- the requested locale translation exists;
- for non-English content, the translation is not older than the English source translation.

Media existence alone never authorizes delivery. Draft, review, archived, unpublished, future-published, missing-locale and stale translations do not authorize it. Missing or unauthorized identifiers return the same 404 response to avoid a private-media existence oracle.

After authorization, delivery reuses one shared EditorialMedia response service that verifies the dedicated disk, allowed object-path prefix, byte size and SHA-256 before returning data. A missing or integrity-invalid object for an otherwise authorized published reference fails as an unavailable dependency with a sanitized server error and no storage details.

Successful responses use:

- the verified stored MIME type;
- `Content-Disposition: inline` with a generated technical filename;
- `X-Content-Type-Options: nosniff`;
- a strong ETag derived from the immutable SHA-256;
- `Cache-Control: public, no-cache, max-age=0, must-revalidate`.

Authorization is evaluated before conditional ETag handling. Caches may retain immutable bytes but must revalidate every reuse, so unpublish or reference removal takes effect on the next request without a positive freshness window. Error responses use `Cache-Control: no-store`.

### Signed preview delivery

Draft/review preview remains authenticated, confirmed-MFA, exact-permission and short-lived signed access.

Preview image URLs are separate short-lived signed URLs scoped to the article, locale, translation and media identifier. The preview-media route proves:

- the authenticated identity still has `wiki.articles.manage`;
- the URL signature and expiration are valid;
- the media is referenced by the current stored translation being previewed.

There is no anonymous draft-media route. Preview responses use private `no-store` caching and the same storage-integrity service as public and EditorialMedia administrator delivery.

### Failure and compatibility boundaries

- No schema migration is required.
- No new permission or wildcard role grant is required.
- No upload processing, format or storage rule changes.
- No public storage symlink or direct object URL.
- No Events or CMS consumer activation.
- No Canary/login-server contract or session change.
- No production, router, DSM or Internet-exposure action.

## Consequences

- Current draft references protect media from deletion, even before publication.
- Historical revision text can outlive media and may become non-restorable until an editor replaces the missing token.
- Public media URLs are stable by immutable media identifier but remain publication-authorized on every request.
- Alternative text can differ correctly by article and locale without duplicating media objects.
- Runtime implementation must add focused tests for parser agreement, transaction rollback, stale-write isolation, effective-publication authorization, signed preview, integrity failure, deletion safety, remote-target neutralization and responsive accessibility.

## Rejected alternatives

- **Expose `storage/app/editorial-media` or `public/storage`:** violates the established private-disk boundary and bypasses publication authorization and integrity verification.
- **Use arbitrary URLs in Markdown:** permits remote tracking, unsafe protocols and interpretation drift.
- **Use the media record `alt_text` as rendered truth:** is not localized or contextual and would silently change article accessibility text outside article revisions.
- **Keep references for every historical revision:** creates indefinite deletion locks for content that is not currently rendered.
- **Attach references only on publication:** allows draft edits and revision restores to race with deletion and makes authoring nondeterministic.
- **Use a positive public cache lifetime:** can continue serving media after unpublish or reference removal until expiry.
- **Grant Wiki editors `media.manage`:** expands upload/deletion authority beyond the task requirement.
