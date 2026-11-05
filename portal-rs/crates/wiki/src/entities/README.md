# World entity wiki

`PORTAL_WIKI_ENTITIES_ENABLED=true` explicitly enables the public world wiki.
Unset, false, malformed and unknown values keep it disabled. Editorial article
routes retain their existing behavior while the world feature is disabled.

Routes are `/wiki/{kind}`, `/wiki/{kind}/{key}` and their `/pl` and `/en`
variants. The enabled home and `/wiki/search` combine world entities with T13
published guides. Search accepts `q`, `kind`, `category`, `classification` and
`page`; lists accept the same filters with their route's fixed kind. Empty
search returns no results unless a type or exact category/classification filter
is supplied. Page numbering starts at 1. World results use 24 rows per page;
editorial search retains T13's 12 rows. Ordering is stable by type then key.

Route slugs map explicitly to T14 families: `items` → `Item`, `creatures` →
`Creature`, `npcs` → `NPC`, `spells` → `Spell`, `quests` → `Quest`, `loot` →
`Loot`, `achievements` → `Achievement`, `areas` → `Area`. Keys remain opaque
and are percent-encoded in links. No family or identity aliases are inferred.

All entity reads go through `WorldStore`'s public API, including allow-list and
source publication vetoes. No active version means unavailable, not an empty
world. Reads spanning an active-version change fail closed. Filter scans use
keyset batches, stop once the requested page and its successor are known, and
fail as unavailable rather than silently truncate a type beyond 100,000 scanned
rows. This is a bounded fallback until the public world API offers indexed
search. No query reads the world's tables directly.

T13 descriptions bind using `wiki_guides.entity_key = "{kind}:{key}"`. Editors
use the existing trusted repository create/review/publish API. Only the current
published revision in the requested locale is rendered, through T13's Markdown
sanitizer. Missing prose is explicit; multiple published bindings are ambiguous
and refuse the response. World updates never rewrite prose. The existing T13
staff form does not create entity bindings; adding that editor control is outside
this task's permitted paths.

The projection recognizes the pinned native source's definition/declaration,
spell bundle, known item presentation and creature authoring profile shapes.
Only selected numeric facts, names, category and classification are rendered.
Unknown item presentation states remain unknown; hidden creature health is
omitted. Source metadata, raw payloads, dialogue and unapproved related entities
are never rendered. Tests use synthetic data rather than copied game fixtures.
