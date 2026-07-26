# ADR 0013: Trusted Wiki Administration Boundary

## Status

Accepted for the bounded Wiki administration slice tracked by `OTERYN-20260726-wiki-administration` and PR #196.

## Context

PR #158 established Wiki persistence, lifecycle services, optimistic locking, append-only revisions, exact permissions and bounded audit. PR #194 added published-only bilingual public reads, restricted CommonMark rendering and search. The remaining trusted-editor workflow requires web administration without duplicating either domain logic or the safe renderer.

The administration surface is security-sensitive because it mutates public content and publication state. It must preserve deny-by-default RBAC, confirmed MFA, CSRF, optimistic locking and bounded audit metadata.

## Decision

### Reuse application services

Administrator controllers remain orchestration-only. Article lifecycle writes call `WikiArticleService`; category writes call `WikiCategoryService`. A narrow `WikiAdminArticleWriter` composes article content writes with featured/order/category presentation state inside one database transaction. A narrow `WikiAdminCategoryWriter` adds deep parent-cycle validation before delegating to the category service.

No controller writes article translations, revisions or lifecycle state directly.

### Exact route permissions

All Wiki administrator routes require:

- `auth`;
- `mfa.confirmed`;
- `wiki.access`.

Mutation groups add the narrow capability:

- `wiki.articles.manage` for draft editing, preview and review submission/return;
- `wiki.categories.manage` for category writes;
- `wiki.publish` for publish, unpublish, archive and revision restore.

No wildcard grant or implicit platform-administrator permission is introduced.

### Conflict semantics

Every article/category update and lifecycle mutation receives the current `lock_version`. A stale or invalid lifecycle operation returns HTTP 409 and does not overwrite newer work. Revision restore remains append-only and creates a new revision linked to the historical source revision.

### Draft preview

Unpublished preview uses a short-lived signed route. The route still requires an authenticated session, confirmed MFA and `wiki.articles.manage`. Stored source Markdown is rendered at request time through the existing restricted `WikiMarkdownRenderer`; persisted or user-supplied HTML is not trusted. Preview responses are marked `noindex,nofollow,noarchive`.

### Presentation metadata

Featured state, display order and category assignments are updated in the same transaction as an article draft write. Category identifiers are validated against Platform-owned Wiki categories. Presentation audit records include only bounded counts, flags, order and lock version—not article bodies or category names.

### Category hierarchy

Category updates reject the category itself, any descendant and pre-existing cycles as a parent. Hierarchy traversal is bounded. The existing database foreign key and optimistic lock remain authoritative for concurrent changes.

## Consequences

- Trusted editors receive a complete bilingual draft/review/publication workflow without a second CMS.
- Public reads continue to expose only approved published content.
- Restore and stale-edit behavior remain deterministic and auditable.
- The admin surface depends on existing shared layout and exact permission middleware.
- EditorialMedia references, initial Wiki content, homepage/SEO integration and staging deployment remain separate child tasks.

## Rejected alternatives

- **Direct model writes in controllers:** would bypass lifecycle, revision and audit invariants.
- **One broad `wiki.manage` permission:** would violate exact deny-by-default authorization.
- **Unsigned or anonymous draft preview:** could leak unpublished content.
- **Rendering editor HTML:** would violate the restricted Markdown trust boundary.
- **Bundling EditorialMedia integration:** would expand path ownership and security scope beyond one reviewable slice.

## Rollback

Revert the application, route, view and test changes. This slice adds no migration and does not modify existing Wiki data contracts.