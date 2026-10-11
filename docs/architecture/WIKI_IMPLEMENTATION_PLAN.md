# Oteryn Wiki Implementation Plan

## Status

**PROPOSED — architecture and delivery plan only.**

This document defines the intended architecture and phased delivery plan for a first-party Oteryn Wiki inside Oteryn Platform. It does not prove that any Wiki capability is implemented.

RubinOT is used only as product inspiration for information architecture and discoverability. Do not copy its code, text, branding, icons, artwork, layouts or proprietary data.

## Goal

Deliver a searchable, bilingual, secure and maintainable knowledge base for Oteryn that is integrated with the existing Laravel application, Identity, Admin/RBAC, Audit and public-site design.

The Wiki must support trusted editors and publishers, published-only public reads, revision history, category navigation, safe media and a clear path to later structured game-data catalogs.

## Architectural decision

Build Wiki as a dedicated module inside the existing Oteryn Platform modular monolith.

Do not deploy a separate MediaWiki, WordPress or external CMS for the initial scope. Do not force Wiki requirements into the existing minimal managed-page model.

Wiki may reuse shared Platform capabilities, but it owns its own content model and workflows.

### Reused Platform capabilities

- Platform Identity and authenticated web sessions;
- confirmed MFA for privileged routes;
- explicit deny-by-default RBAC;
- administrator audit primitives;
- Blade layout and shared frontend components;
- Platform database and migration lifecycle;
- cache and queue infrastructure when justified;
- public game-data query services for dynamic status widgets.

### Wiki-owned capabilities

- Wiki articles and translations;
- categories and category navigation;
- revisions and restore workflow;
- publication lifecycle;
- Wiki search;
- related articles;
- redirects after slug changes;
- Wiki-specific media references and safe upload workflow;
- Wiki metadata, SEO and sitemap generation.

## Scope for the first production-capable release

### Public surfaces

- `/wiki` entry point;
- Polish and English content;
- Wiki homepage;
- category and subcategory navigation;
- published article pages;
- breadcrumbs;
- generated table of contents;
- related articles;
- featured content;
- global Wiki search with keyboard shortcut;
- responsive desktop, tablet and mobile layouts;
- accessible keyboard and focus behavior;
- correct not-found and unpublished states;
- canonical URLs, `hreflang`, metadata and sitemap entries.

### Administrative surfaces

- Wiki dashboard;
- article list with status, locale and category filters;
- create and edit article;
- Markdown preview;
- draft, review, published and archived states;
- publish and unpublish actions;
- category and subcategory management;
- revision history;
- restore a historical revision by creating a new revision;
- preview unpublished content through a short-lived signed URL;
- related-article management;
- featured-content selection;
- safe image library;
- audit visibility through the existing administrator audit surface.

## Explicit non-goals for the initial release

- public or player-authored editing;
- article comments;
- arbitrary raw HTML;
- arbitrary iframe or script embedding;
- plugin installation or executable uploads;
- automatic import of all legacy engine or client data;
- interactive world map;
- fully structured item, monster, quest or NPC database;
- AI-generated content publication without human review;
- webshop, premium currency, Battle Pass or character bazaar.

## Domain and data model

The exact schema must be confirmed by an implementation ADR and migration review. The following model is the target boundary.

### `wiki_articles`

Represents one language-independent article identity.

Suggested fields:

- `id`;
- `content_type` such as `guide`, `system`, `quest`, `item`, `event` or `reference`;
- lifecycle status;
- featured flag and sort order;
- optional icon media reference;
- author, last editor and publisher Identity references;
- `published_at`;
- optimistic-lock version;
- timestamps.

### `wiki_article_translations`

Represents the localized public and editable content.

Suggested fields:

- article reference;
- locale;
- title;
- slug;
- summary;
- source Markdown;
- safe rendered representation or render version marker;
- SEO title and description;
- normalized search document;
- timestamps.

Required constraint:

```text
UNIQUE(locale, slug)
```

### `wiki_categories`

Represents the language-independent category tree.

Suggested fields:

- `id`;
- `parent_id`;
- stable key;
- optional icon media reference;
- sort order;
- visibility;
- timestamps.

### `wiki_category_translations`

Suggested fields:

- category reference;
- locale;
- name;
- slug;
- description.

### `wiki_article_category`

Many-to-many relation between articles and categories with optional ordering.

### `wiki_revisions`

Append-only snapshots for one article translation.

Suggested fields:

- article and locale;
- revision number;
- title, summary and source Markdown snapshot;
- editor Identity;
- change note;
- source revision for restore operations;
- timestamp.

Existing revisions must never be updated in place. Restoring an older revision creates a new revision.

### `wiki_related_articles`

Explicit ordered relationships between articles. Prevent self-reference and duplicate directed relations.

### `wiki_redirects`

Maps an old localized slug to a current article after an approved slug change.

Redirect creation must prevent loops and conflicts with active slugs.

### `wiki_media`

Stores metadata and ownership for approved media objects, not raw file data in database rows.

Suggested fields:

- storage disk and path;
- generated technical filename;
- original display name where safe;
- MIME type;
- byte size;
- image dimensions;
- SHA-256 digest;
- uploader Identity;
- alternative text per locale or a separate translation table;
- lifecycle status;
- timestamps.

## Content format and rendering

Use a restricted Markdown profile rather than arbitrary HTML.

The supported profile may include:

- headings;
- paragraphs;
- emphasis;
- lists;
- tables;
- block quotes;
- safe links;
- images selected from the Wiki media library;
- code blocks where needed;
- explicit callout components;
- generated heading anchors and table of contents.

The renderer must reject or neutralize:

- raw HTML unless a separately reviewed allowlist is introduced;
- scripts and event handlers;
- iframes and embedded forms;
- dangerous URL protocols;
- untrusted inline styles;
- remote image hotlinking when not explicitly approved.

Rendering rules and sanitizer behavior are security-critical and require regression tests.

## Public routes

Proposed route shape:

```text
GET /wiki
GET /wiki/{locale}
GET /wiki/{locale}/search
GET /wiki/{locale}/category/{slug}
GET /wiki/{locale}/{slug}
```

`/wiki` should resolve the supported locale from an explicit user choice or a deterministic default. Do not create uncontrolled locale-dependent duplicate URLs.

## Administrative routes

Proposed route groups:

```text
GET    /admin/wiki
GET    /admin/wiki/articles
GET    /admin/wiki/articles/create
POST   /admin/wiki/articles
GET    /admin/wiki/articles/{article}/edit
PUT    /admin/wiki/articles/{article}
POST   /admin/wiki/articles/{article}/submit-review
POST   /admin/wiki/articles/{article}/publish
POST   /admin/wiki/articles/{article}/unpublish
POST   /admin/wiki/articles/{article}/archive
GET    /admin/wiki/articles/{article}/revisions
POST   /admin/wiki/articles/{article}/revisions/{revision}/restore

GET    /admin/wiki/categories
POST   /admin/wiki/categories
PUT    /admin/wiki/categories/{category}

GET    /admin/wiki/media
POST   /admin/wiki/media
DELETE /admin/wiki/media/{media}
```

Exact route and action shape should follow current repository conventions and remain REST-like where practical.

## Authorization model

Every privileged Wiki route must combine:

- `auth`;
- `mfa.confirmed`;
- one exact Wiki permission.

Proposed permissions:

- `wiki.access`;
- `wiki.articles.manage`;
- `wiki.categories.manage`;
- `wiki.media.manage`;
- `wiki.publish`.

Suggested role bundles:

### `wiki_editor`

- Wiki access;
- create and edit drafts;
- manage approved media;
- no publication authority.

### `wiki_publisher`

- editor capabilities;
- publication and unpublication;
- revision restore;
- category management.

Existing roles must not receive new permissions through a wildcard. Any update to `platform_admin` or another role bundle must be explicit and reviewed.

## Audit requirements

Append administrator audit events for at least:

- article create and update;
- review submission;
- publish and unpublish;
- archive;
- revision restore;
- slug change and redirect creation;
- category create, update and reorder;
- media upload, replacement and deletion;
- featured-content change.

Audit metadata must remain minimal, non-secret and bounded. Article bodies and media bytes do not belong in the audit record.

## Search architecture

Define a module interface so the storage engine can change without changing controllers or domain workflows.

Example responsibility:

```text
WikiSearch
- search(locale, query, filters, page)
- index(articleTranslation)
- remove(articleTranslation)
```

### Initial implementation

Use a database-backed search implementation if it provides acceptable relevance and predictable test behavior for the initial content volume.

Ranking should prioritize:

1. exact and prefix title matches;
2. summary matches;
3. category matches;
4. body matches.

Search requirements:

- published content only;
- locale isolation;
- pagination;
- bounded query length;
- abuse-oriented rate limiting;
- deterministic ordering for equal scores;
- safe result snippets;
- no draft leakage.

A future migration to Meilisearch or another maintained engine must remain an implementation replacement behind the interface, not a rewrite of Wiki business logic.

## Media security

Initial accepted formats should be limited to raster images such as JPEG, PNG and WebP.

Reject at minimum:

- SVG;
- HTML;
- executable files;
- archives;
- office documents;
- files whose decoded content does not match the detected MIME type.

Required processing:

1. server-side size limit;
2. actual content and MIME inspection;
3. image decode;
4. dimension and decompression-bomb limits;
5. re-encode using a maintained image library;
6. metadata removal;
7. generated filename;
8. digest calculation;
9. storage outside executable paths;
10. authorization and audit;
11. block deletion while referenced by published content unless a safe replacement flow is used.

Do not add upload support by merely exposing the current CMS storage path.

## Caching and freshness

Wiki content may be cached because publication is explicit and infrequent.

Suggested cache boundaries:

- localized article by slug and revision;
- localized category tree;
- localized Wiki homepage;
- featured and recently updated lists.

Publishing, unpublishing, restoring, recategorizing or changing a slug must invalidate the affected entries after the database transaction commits.

Dynamic world status and online counts must be requested from PublicGameData separately. Do not freeze runtime status inside long-lived Wiki page caches.

## Internationalization

Initial supported locales:

- English (`en`);
- Polish (`pl`).

Requirements:

- one language-independent article identity;
- explicit translation state;
- visible indication when a translation is missing or older than the source revision;
- localized slugs;
- `hreflang` links between published translations;
- no automatic publication of machine translations.

## User experience

### Desktop

- persistent category sidebar;
- prominent search;
- readable article column;
- optional article table of contents;
- related links and update metadata.

### Tablet

- collapsible sidebar;
- preserved search prominence;
- responsive tables and callouts.

### Mobile

- drawer or disclosure navigation for categories;
- visible search action;
- collapsible table of contents;
- no horizontal page overflow;
- readable tap targets and focus order.

The visual language must use Oteryn assets and components. Decorative fantasy elements must not reduce reading contrast or information density.

## Initial information architecture

```text
Getting Started
- Download
- Installation
- Creating an Account
- Creating a Character
- First Login

Server Information
- Rates
- PvP Rules
- Server Save
- Commands
- Supported Client

Game Systems
- Vocations
- Skills
- Spells
- Tasks
- Oteryn-specific systems

World
- Cities
- Hunting Places
- Quests
- Monsters
- Bosses

Items
- Weapons
- Equipment
- Backpacks
- Consumables
- House Items

Community
- Events
- Guilds
- Rules
- Discord

Support
- FAQ
- Known Issues
- Account Security
- Contact and Bug Reports
```

Only categories backed by real Oteryn content should be published.

## Delivery slices

The implementation must be delivered as small reviewed slices. Do not implement the whole programme in one pull request.

### Slice 0 — architecture and contracts

- create the durable ADR for Wiki module, content format and ownership;
- update module catalog with `Wiki: IMPLEMENTING` only when implementation begins;
- confirm exact dependency and package choices from the repository;
- record media and Markdown threat assumptions;
- create the implementation task and owned paths.

### Slice 1 — domain and persistence foundation

- migrations and models for articles, translations, categories and revisions;
- lifecycle and invariant services;
- exact RBAC permissions;
- audit event definitions;
- factories and database tests;
- no public activation yet.

### Slice 2 — published public read path

- Wiki homepage;
- article and category reads;
- published-only queries;
- breadcrumbs and navigation;
- safe Markdown rendering;
- 404 and unpublished behavior;
- focused security tests.

### Slice 3 — editor workflow

- article list;
- create and edit draft;
- preview;
- optimistic locking;
- review state;
- authorization and audit tests.

### Slice 4 — publication and revisions

- publish, unpublish and archive;
- append-only revision history;
- restore by creating a new revision;
- signed private preview;
- slug redirects.

### Slice 5 — categories and related content

- category tree administration;
- ordering;
- related articles;
- featured content;
- cache invalidation.

### Slice 6 — search

- search interface and database implementation;
- title-weighted ranking;
- category filtering;
- keyboard search palette;
- no-draft-leak tests;
- query abuse limits.

### Slice 7 — safe media

- media library;
- upload processing and re-encoding;
- alt text;
- article insertion;
- reference-safe deletion;
- security regression tests.

### Slice 8 — bilingual, SEO and accessibility completion

- PL and EN workflow;
- translation freshness markers;
- canonical and `hreflang` metadata;
- sitemap;
- desktop, tablet and mobile completion;
- keyboard, focus and contrast validation.

### Slice 9 — launch content and activation

- minimum approved launch article set;
- add Wiki to public navigation and homepage;
- production-like migration and rollback validation;
- full functional, visual and accessibility acceptance;
- no production readiness promotion without direct production evidence.

## Minimum launch content

Before public activation, publish reviewed English and Polish versions of at least:

1. Download and Installation;
2. Creating an Account;
3. Creating a Character;
4. First Login;
5. Server Information;
6. Server Rates;
7. Vocations;
8. PvP and Game Rules;
9. Account Security and MFA;
10. FAQ;
11. Known Issues;
12. Discord and Support;
13. Report a Bug.

## Validation strategy

Each slice must use the smallest relevant validation from repository evidence. The complete programme should include:

- migration and rollback tests;
- model and domain invariant tests;
- RBAC and confirmed-MFA denial tests;
- audit tests;
- Markdown XSS and unsafe-link regression tests;
- draft and unpublished leakage tests;
- search locale and publication-state tests;
- upload MIME, decode, dimension and reference tests;
- concurrent edit conflict tests;
- cache invalidation tests;
- browser E2E for public navigation, editor workflow and publication;
- responsive desktop, tablet and mobile acceptance;
- keyboard/focus accessibility;
- production-like existing-data migration and rollback evidence before release.

Do not claim tests or CI have passed until verified on the exact current head.

## Risks and required decisions

The implementation task must resolve and document:

- the maintained Markdown parser and safe rendering configuration already available or approved for addition;
- the image processing library and runtime support available in the deployed PHP image;
- storage disk and serving topology for Wiki media;
- database search capability and locale behavior in the actual MariaDB version;
- whether English or Polish is the canonical editorial source language;
- translation staleness rules;
- content ownership and approval process;
- backup and restore coverage for media objects in addition to database rows.

Unknowns must remain explicit and must not be converted into assumptions.

## Definition of done

The Wiki programme is complete only when:

- public Wiki homepage, category pages, search and article pages work;
- only published localized content is public and searchable;
- editors can create and revise drafts;
- publishers can publish, unpublish, archive and restore revisions;
- all privileged routes require authentication, confirmed MFA and an exact permission;
- every privileged mutation is audited;
- Markdown rendering and media upload boundaries are security-tested;
- bilingual URLs and metadata are correct;
- desktop, tablet and mobile UX pass acceptance;
- minimum launch content is approved and published;
- migration, rollback and backup boundaries are documented and validated;
- the public navigation link is activated through a separately reviewed final slice.
