# ADR 0015 — Wiki launch-content provisioning

## Status

Accepted — 2026-07-26

## Context

ADR 0010 made Wiki content Platform-owned, bilingual, revisioned and explicitly published. The public Wiki and administration slices now enforce publication freshness, safe Markdown, exact permissions, confirmed MFA on administrator HTTP routes, optimistic locking and bounded audit.

The approved Wiki plan requires thirteen English and Polish launch topics. Repository evidence proves the Platform workflows and a small set of product facts, but it does not prove current server-rate values, detailed vocation mechanics, complete PvP/rules text, a Discord destination or the final authoritative game-client login rollout.

Launch content is mutable editorial data. A schema migration that silently inserts or overwrites published copy would bypass the ordinary named-publisher workflow and make later editorial changes unsafe.

## Decision

### 1. The launch package is versioned source

`WikiLaunchContentCatalog` owns one reviewed bilingual package and an explicit content version. Each article records the repository files that support its substantive claims.

The package contains the thirteen minimum launch topics. Where product facts remain unknown, the article names the current authoritative public surface or explicitly says that no approved value is present. It does not substitute Canary examples, another server's behavior or an inferred default.

### 2. Installation is an explicit operator action

The package is installed with:

```text
php artisan wiki:launch-content:install <publisher-email> --content-version=<exact-version>
```

The version option is an explicit acknowledgement of the reviewed package. Installation is not part of ordinary schema migration.

The named Identity must:

- exist and be enabled;
- have confirmed MFA;
- hold `wiki.access`;
- hold `wiki.categories.manage`;
- hold `wiki.articles.manage`;
- hold `wiki.publish`.

No role or wildcard permission is added. Confirmed MFA is verified as durable Identity state because a console command has no administrator browser session.

### 3. Existing Wiki services remain authoritative

The installer creates categories and articles through the existing category writer, article writer and lifecycle service. Therefore initial content receives the same:

- exact-permission checks;
- bilingual validation;
- append-only revisions;
- draft-to-review-to-published lifecycle;
- bounded administrator audit events;
- transactional persistence;
- current EditorialMedia reference synchronization.

The content package contains no media reference.

### 4. Installation is atomic and conflict-safe

One stable Wiki permission row serializes concurrent package installations. The complete operation runs in one database transaction.

Before creating missing records, the installer reconciles every catalog category and article:

- an absent record may be created;
- an existing exact published record is an idempotent no-op;
- a reused key or localized slug with different ownership is a conflict;
- changed content, presentation, categories, locale set or publication state is a conflict.

A conflict aborts the whole transaction. The installer never overwrites, republishes or normalizes an editor's later change.

Unrelated Wiki content is preserved.

### 5. Public content points to live authority where values can change

The launch articles link to route-backed Download, account, character, server-information, rules, announcements and support surfaces. Runtime values are not copied into static Wiki content when their authoritative source can change independently.

English remains the canonical editorial source and Polish is installed in the same transaction after English, preserving the public freshness rule.

## Consequences

- The launch set can be reviewed as code and installed deliberately on a target environment.
- Repeated exact installation is safe and produces no new revisions or audit events.
- Existing editorial changes cannot be silently replaced by a deployment.
- A future content revision requires a new reviewed package version or an ordinary trusted-editor workflow.
- Database migration and rollback remain schema-only; removing the code does not delete published editorial data.
- Exact target-environment installation and smoke remain deployment evidence, not production proof.

## Rejected alternatives

### Publish content from a migration

Rejected. It would conflate schema evolution with a privileged editorial mutation and complicate rollback after editors change the content.

### Upsert the package on every deployment

Rejected. It would overwrite later editorial work and bypass optimistic-locking intent.

### Use a synthetic system publisher

Rejected. Initial publication must remain attributable to one existing, MFA-confirmed Identity with every exact Wiki permission.

### Fill missing gameplay facts from Canary examples

Rejected. Configuration examples and generic upstream defaults are not Oteryn product approval.

### Leave required topics absent

Rejected. A short source-authority article truthfully identifies an unknown and the official place to obtain an updated answer without inventing the answer.
