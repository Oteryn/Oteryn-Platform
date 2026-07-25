# ADR 0011: Safe editorial media boundary

- Status: Accepted
- Date: 2026-07-25

## Context

Wiki, Events and CMS need reusable editorial images, but the Platform previously had no upload-security boundary and its existing local/public disks were not suitable for untrusted image ingestion. The public-site and Wiki plans require raster-only content verification, decode and re-encode, metadata removal, immutable names, bounded dimensions, reference tracking and deletion safety.

An upload is a security-critical browser-to-storage trust transition. The implementation must not create an executable upload surface, expose arbitrary files through the public symlink, couple media ownership to one consumer, or assume storage success.

## Decision

Introduce a Platform-owned `EditorialMedia` module that owns normalized image records, private objects and bounded consumer references. Wiki, Events and CMS may consume this module in later separately owned changes; this ADR does not activate those integrations.

### Accepted formats and processing

Only JPEG, PNG and WebP are accepted. The original filename extension, `fileinfo` MIME result and image header MIME must agree. The file must satisfy configured byte, width, height and decoded-pixel limits before full decode.

The server first requires an exact JPEG, PNG or WebP container boundary, so trailing polyglot/script payloads are rejected. It then decodes the image with the deployed PHP GD extension and re-encodes it in the same raster format. Only the new pixel-derived output is retained. This removes EXIF and other source metadata and proves that the accepted object can be decoded by the runtime. Missing codecs, malformed content, invalid configuration and encoding failures fail closed.

SVG, HTML, scripts, executables, archives, office documents and every other file type are rejected. Client-provided MIME is not trusted as evidence. Animated PNG/WebP behavior is not a promised capability; the stored trust object is the static pixel result produced by the maintained decoder.

No external malware scanner is added for this raster-only slice because source bytes are format-verified, decoded, re-encoded and discarded before private storage. This decision must be revisited before original-file retention or any non-raster format is introduced.

### Storage

Normalized objects use 192-bit random immutable technical names under a dedicated `editorial_media` Laravel disk rooted at `storage/app/editorial-media`. The disk is private, is not part of `public/storage`, and has filesystem exceptions enabled. The database records the disk, path, verified MIME, normalized extension, byte size, dimensions, SHA-256, uploader and bounded alternative text. Original display names are optional sanitized metadata and are never used as storage paths.

The existing Synology deployment persists the complete Platform storage directory, so this directory follows the same persistent volume boundary. The deployed PHP configuration explicitly permits the application’s 8 MiB media boundary while bounding POST size, upload count, memory and execution time. Production backup and restore must include both Platform database records and editorial media objects; repository configuration alone does not prove that production backup coverage exists.

### Thumbnails

A thumbnail is generated only when an image exceeds the configured administrator-preview dimension. It preserves aspect ratio, never upscales, uses the same verified format, is also re-encoded and receives its own byte-size and SHA-256 integrity record. Small images reuse the normalized original for administrator preview. This avoids derivative proliferation while bounding administration-page transfer and decode work for large images.

### Authorization and audit

Every library route requires `auth`, `mfa.confirmed` and the exact `media.manage` permission. The migration explicitly grants this permission to the existing `content_editor` and `platform_admin` role bundles; it is not a wildcard and no other role gains it.

Uploads and successful deletions append bounded administrator audit events containing technical integrity metadata only. Image bytes, alternative text, original file content and secrets are excluded from audit metadata.

### References and deletion

`editorial_media_references` records one bounded usage slot for a known consumer (`cms`, `events` or `wiki`). Consumers must use the reference manager, which locks the media row while attaching or moving a slot.

Deletion locks the media row and refuses while any reference exists. A database `RESTRICT` foreign key independently prevents direct deletion of referenced media. Unreferenced deletion verifies that every recorded private object exists and can be read before mutation. Storage deletion failure rolls back the database operation; already removed objects are restored from bounded in-memory backups when a later step fails.

### Serving boundary

This slice exposes image bytes only through administrator routes protected by the same exact permission. Responses use the verified MIME, `Content-Disposition: inline`, private caching and `X-Content-Type-Options: nosniff`. Public serving and consumer-specific authorization are separate future integration decisions.

## Rejected alternatives

- **Use the public disk or storage symlink:** exposes untrusted objects through a broad public path and does not fail closed.
- **Store original uploads and trust validation rules:** retains metadata/polyglot payloads and does not prove decoder safety.
- **Accept SVG after sanitization:** creates an unnecessary active-content parser and sanitizer boundary.
- **Add a generic document or plugin library:** violates least privilege and introduces executable/arbitrary-file risk.
- **Make the library Wiki-owned:** prevents clean reuse by Events and CMS and couples storage lifecycle to one consumer.
- **Add a new Composer image package:** not required for this bounded operation; the controlled PHP runtime can provide maintained JPEG/PNG/WebP GD codecs without another application dependency.

## Consequences

- CI and deployed Platform images must include GD with JPEG, PNG and WebP support.
- The database migration is additive and reversible, but rollback must occur only after media references/records and private objects are handled operationally.
- Public consumers need separate authorization, route, cache and publication reviews before using this library.
- Initial alternative text is one bounded value per media object. Localized alternative text may later be added as a separate translation record without changing object identity.
- Re-encoding intentionally removes source orientation metadata. Editors must verify the normalized administrator preview and provide correctly oriented pixel data.
- Private orphan cleanup after catastrophic storage restoration failure remains an operational incident path; the normal application path fails closed and never exposes those objects publicly.
