# ADR 0010 — Wiki module and persistence foundation

## Status

Accepted — 2026-07-24

## Context

The Oteryn Wiki requires localized slugs, a controlled publication lifecycle, optimistic concurrency protection, append-only content history and later safe Markdown rendering. The existing managed-page model intentionally provides only a small plain-text CMS surface and does not satisfy those invariants without coupling unrelated content workflows.

PR #142 accepted the dedicated Wiki architecture and phased delivery plan. PR #146 reserved the exact first-slice Wiki permission keys through the shared extension mechanism without granting them to any role.

This first implementation slice must establish durable ownership and persistence without activating public Wiki routes, navigation, rendering, media, search or player editing.

## Decision

### 1. Wiki is a dedicated module in the existing modular monolith

Wiki implementation lives under `App\Wiki` and owns its application rules, domain vocabulary and persistence models.

Wiki may depend on shared Platform Identity references, exact Admin/RBAC authorization and the administrator audit recorder. CMS does not own Wiki persistence, and Wiki does not alter the generic managed-page model.

### 2. Wiki persistence is Platform-owned

Oteryn Platform owns these first-slice tables and their reversible Laravel migrations:

- `wiki_articles`;
- `wiki_article_translations`;
- `wiki_categories`;
- `wiki_category_translations`;
- `wiki_article_category`;
- `wiki_revisions`.

No Canary or login-server schema, credential or runtime contract changes.

### 3. Localization is explicit

The first supported locales are exactly:

- English (`en`);
- Polish (`pl`).

Article and category translations are unique by language-independent owner plus locale. Public-facing slugs are unique by `(locale, slug)`, so equivalent slugs may exist in different locales but never twice in one locale.

Both English and Polish article translations must exist and contain a title, slug, summary and source Markdown before publication. Draft and review content may remain incomplete.

### 4. Lifecycle transitions are deterministic

Article status uses this vocabulary:

- `draft`;
- `in_review`;
- `published`;
- `archived`.

Allowed transitions are:

```text
draft      -> in_review | archived
in_review  -> draft | published | archived
published  -> draft | archived
archived   -> no transition in this slice
```

A published or archived article cannot be edited directly. Unpublishing returns an article to draft before further editing.

### 5. Content edits use optimistic locking

Articles and categories carry a monotonic `lock_version`.

Every supported update receives the caller's expected version, locks the current row in the transaction and fails with an explicit stale-edit error if the durable version changed. A stale edit never silently overwrites newer content.

### 6. Revisions are append-only snapshots

Creating article content, changing a localized article translation and restoring historical content each append a `wiki_revisions` row.

A revision stores the bounded editorial snapshot needed to reconstruct that localized version, including its revision number and article version. Existing revisions are not updated or deleted through the supported model.

Restoring a revision:

1. requires publication authority;
2. is allowed only while the article is editable;
3. copies the selected snapshot into the current translation;
4. increments the article lock version;
5. appends a new revision referencing the source revision.

Lifecycle-only transitions do not duplicate unchanged article bodies into revision storage; they are represented by bounded administrator audit events.

### 7. Source content is restricted Markdown, not HTML

The first slice stores source Markdown only. Raw HTML and dangerous URL protocols are rejected at the model/domain boundary.

This decision does not select or activate a Markdown renderer. Parser, sanitizer, rendered-cache and public-output behavior remain a later security-reviewed slice. No arbitrary HTML field is added.

### 8. Authorization uses only reserved exact permissions

The module uses:

- `wiki.access`;
- `wiki.articles.manage`;
- `wiki.categories.manage`;
- `wiki.publish`.

There is no wildcard. Reservation is not authorization. This slice adds no role-permission rows, including for `platform_admin`.

Future privileged HTTP routes must compose:

```text
auth
+ mfa.confirmed
+ admin.permission:<one exact Wiki permission>
```

No Wiki HTTP route is activated by this ADR.

### 9. Wiki audit events use bounded metadata

Wiki application services append administrator audit events in the same Platform transaction as successful mutations where practical.

Metadata may contain identifiers, status, locale, lock version, revision number and source-revision reference. It must not contain complete article bodies, complete category descriptions, credentials, secrets or personal data.

## Consequences

- Wiki requirements do not expand or destabilize the existing managed-page persistence.
- Database uniqueness and application validation jointly protect localized slugs and locales.
- Editors receive no publication authority implicitly.
- Concurrent stale edits fail instead of overwriting newer content.
- Content history is reconstructable through append-only revisions.
- Public Wiki activation, rendering safety, search, redirects, media and user-facing administration remain later reviewed slices.
- Existing deployment migration/rollback validation will exercise the additive schema in required CI; direct production proof remains separate.

## Rejected alternatives

### Extend managed pages for Wiki

Rejected. Generic managed pages do not own localized identity, category relations, optimistic locking or append-only localized revisions.

### Publish with one available locale and silently fall back

Rejected for the first release. Silent fallback can expose incomplete or stale editorial meaning. Both supported locales are required before publication.

### Mutable revision rows

Rejected. Editing or deleting historical snapshots would make restore and audit evidence unreliable.

### Last-write-wins editing

Rejected. It silently discards newer editorial work.

### Grant all new Wiki permissions to existing platform administrators automatically

Rejected. `platform_admin` is an explicit bundle, not a wildcard, and future authority requires a reviewed role-permission decision.

### Activate placeholder routes

Rejected. The public extension contract prohibits placeholder routes and links. Public Wiki reads begin only in a later complete slice.
