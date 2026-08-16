# OTERYN-20260816 — CONTENT-AUDIT handoff to coordinator

```yaml
audit_id: OTERYN-20260816-content-audit
repository: blakinio/Oteryn-Platform
base_sha: 286efb1625d510c9d2cc344cb51a2438b31ebe48
parent_issue: 1115
audit_pr: 1117
runtime_access: false
new_remediation_scaffolds: 0
ledger_markdown: docs/agents/reports/OTERYN-20260816-content-audit-ledger.md
ledger_json: docs/agents/reports/OTERYN-20260816-content-audit-ledger.json
```

## Barrier result

The discovery barrier is complete at repository-evidence level. The audited `main` is not an empty content shell, but repository capability and production population must remain separate.

Proven current-state anchors:

- Game Catalog has public/admin/import/activation/query machinery and focused tests, but the only record-level corpus proven inside the audited repository for public projection is explicitly synthetic fixture data. Production active profile/snapshot and visible counts remain `UNKNOWN`.
- Wiki has a real reviewed deployable launch corpus, not only schema/CRUD: inventory version `2026-08-10.2`, catalog version `2026-07-26.1`, reviewed catalog blob `07ff3324a4530958f9f4e164c5f7a2a399a1bb8b`, exactly 4 categories and 13 bilingual articles. The install test yields 8 category translations, 26 article translations, 26 revisions and 13 published articles. Production installation remains `UNKNOWN`.
- Player Companion currently proves one complete route/test family: the authenticated Hunt Session Analyzer. No additional companion tool route family is proven on the audited base.
- `routes/web.php` deterministically loads 13 `routes/modules/*.php` files. All 13 have an explicit ledger disposition.
- Account/security/registration/login/recovery/MFA are implemented workflow surfaces and are not missing static-content families.
- Billing/library tiers are not a product surface proven on this base and are outside Issue #1115; do not invent a content lane for them.

## Corrected evidence

Do **not** use the earlier provisional counts `9 guide steps + 2 replies` or `Wiki 10 + 9 pages`. They were not substantiated by exact repository evidence during this audit and are rejected in `CA-020`.

Use these exact Wiki counts instead:

```yaml
wiki_launch_inventory_version: 2026-08-10.2
wiki_launch_catalog_version: 2026-07-26.1
wiki_launch_catalog_blob: 07ff3324a4530958f9f4e164c5f7a2a399a1bb8b
wiki_categories: 4
wiki_articles: 13
wiki_category_translations_after_install: 8
wiki_article_translations_after_install: 26
wiki_revisions_after_install: 26
wiki_published_articles_after_install: 13
wiki_production_installation: UNKNOWN
```

Game Catalog guide/reply production counts remain `UNKNOWN`.

## Source-material boundary

Draft PR #1116 is still unmerged and therefore candidate evidence, not trusted `main`. Its owner-supplied archive identity is:

```yaml
archive_sha256: 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26
default_profile: data-global
alternate_profile: data-crystal
```

High-value candidate source counts that were re-read from #1116:

- items: 17,571 XML definitions / 38,059 expanded unique server IDs — `DIRECT_STRUCTURED`;
- creatures: 1,802 Lua files — `TRANSFORM_REQUIRED`;
- creature loot: 1,754 loot-bearing files — `TRANSFORM_REQUIRED`;
- Bestiary-like metadata: 799 files — `TRANSFORM_REQUIRED`;
- NPCs: 1,112 Lua files — `TRANSFORM_REQUIRED`;
- structured NPC shops: 290 files / 10,907 buy-or-sell rows — `TRANSFORM_REQUIRED`;
- NPC world entries: 1,043 — direct structured with profile caveat;
- player spells: 218 files — `TRANSFORM_REQUIRED`;
- achievements: 558 entries — direct structured for core fields;
- quests: 1,061 files / 119 top-level families — `PARTIAL_SEMANTICS`;
- imbuements: 72, mounts: 250, outfits: 260, vocations: 11 — `DIRECT_STRUCTURED` candidates.

These are source-side counts only. They are not Platform production counts and they do not grant native Oteryn authority or publication rights.

## Live ownership locks

Refresh these before dispatch, but at audit closeout they are:

| Owner | State | Lock / consequence |
|---|---|---|
| #1114 | open draft | owns PublicPortal Today runtime paths; do not overlap |
| #1116 | open draft | programme/source-inventory bootstrap candidate; unmerged |
| #330 | open programme/issue | existing Game Catalog production-completion owner |
| #489 | open issue | broad catalogue/content/knowledge/tools completion owner |
| #301 | open blocked issue | spells/NPCs/quests/achievements package ownership |
| #338 | open draft | NPC/shop schema 1.3 consumer; producer/authority hold; no duplicate consumer work |
| #1115 | open owner-started programme | coordinator umbrella for this content-completion effort |

Because these owners already cover the material gaps, `CONTENT-AUDIT` created zero new task/branch/PR remediation scaffolds.

## Recommended coordinator wave

### Barrier A — read-only runtime truth

When separately authorized, assign a runtime-evidence task that performs **no mutation** and records:

1. Game Catalog public profile existence/enabled state;
2. active snapshot identity/schema/content target;
3. visible item/creature/loot counts and any guide/reply counts exposed by the active snapshot;
4. Wiki exact expected-inventory state against version `2026-08-10.2` / 4 categories / 13 articles;
5. runtime configuration only where needed for player-visible reachability, e.g. `marketplace.enabled`.

If runtime access is not authorized, keep these fields `UNKNOWN`; do not convert them to `EMPTY`.

### Barrier B — reuse existing catalogue owners

- Catalog core item/creature/loot/Bestiary work routes through #330/#489 and current immutable schema/activation semantics.
- NPC/shop work must reuse or wait for #338; do not create a competing schema 1.3 consumer.
- Spells/NPCs/quests/achievements must reconcile #301 before any new slice.

### Barrier C — bounded player value

After a structured source contract is accepted, choose one Player Companion vertical slice with independent paths. Strong candidates are imbuement planning or equipment/item comparison because their source families are structured, but authority/product fit must still be proven at dispatch time. Do not create a mega-PR for the whole toolbox.

### Barrier D — Wiki reference expansion

Generate/link reference surfaces only from accepted structured facts. Preserve source/archive/profile/transformation provenance. Keep editorial guides/lore/walkthrough prose separate and do not bulk-copy third-party descriptions/dialogue/assets/maps/media.

## Serialization keys for coordinator state

```yaml
content_audit_base_sha: 286efb1625d510c9d2cc344cb51a2438b31ebe48
wiki_inventory_version: 2026-08-10.2
wiki_catalog_version: 2026-07-26.1
wiki_catalog_blob_sha: 07ff3324a4530958f9f4e164c5f7a2a399a1bb8b
source_archive_sha256: 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26
source_profile_default: data-global
source_profile_alternate: data-crystal
catalog_runtime_population: UNKNOWN
wiki_runtime_installation: UNKNOWN
player_companion_proven_vertical_slices: 1
new_audit_scaffolds: 0
```

## Unresolved decisions / missing evidence

1. Whether any CrystalServer-derived family may become accepted durable Platform authority rather than reference/provenance input.
2. Whether third-party-derived prose/assets/dialogue/maps/media have sufficient publication authority; the archive license alone does not settle every embedded right.
3. Current production/staging content and feature-config state, because this audit has repository authority only.
4. Which single Player Companion tool should be first after dependency/authority refresh.

These uncertainties block only the affected lane. Independent existing work such as #1114 may continue under its own ownership.
