# Portal search (T48)

`GET /search`, `/pl/search` and `/en/search` render the shared component system
with Polish/English Fluent catalogs. Editorial and world queries use 2–80
characters after whitespace normalization. Character queries use the public
profile name rules, including a 64-byte limit. Control characters are rejected
and `page` is 1–1000.
Unknown parameters, types and malformed pagination answer 422. Database/source
failure answers 503. Empty searches do not enumerate documents. Search responses
are no-store/noindex. The existing locale/source WikiSearch limiter applies.

`kind` selects `news`, `pages`, `events`, `wiki`, `world` or `characters`.
Omitting it searches editorial sources together. Paged sources return up to 20
results, with an extra candidate to detect the next page and deterministic ties.
Editorial relevance weights titles above body text. PostgreSQL `simple` full-text
queries are parameterized; wildcards and SQL syntax are never executed.

Governing references:

- `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`, sections 4, 5.1, 6.1 and T48.
- `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`, character
  identity, current visibility, freshness and independent privacy floor.
- `docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`,
  deny on missing freshness/privacy evidence. A proposal grants no extra fields.
- Laravel public wiki search provides the 422/503 behavior reference. Publication
  predicates mirror the current Rust news/pages/events/wiki public readers.

The owning editorial crates depend on web, so adding reverse dependencies would
create a cycle. `search.sql` is the narrow read-only query seam, with current
publication and translation checks rather than a separately maintained index.
No cached document can make unpublished content eligible.

World search requires `PORTAL_WIKI_ENTITIES_ENABLED=true` (off by default).
Candidate queries read only keys and match presentation names; the binary injects
`WorldStore::get_entity` to recheck current public eligibility. Current approved
name/key matching is repeated after this read, so a changed version cannot retain
an obsolete name match. Source JSON and arbitrary descriptions are never rendered.

Character search requires `PORTAL_PUBLIC_PROFILES_ENABLED=true` (off by
default). The injected community provider matches a full name case-insensitively
and preserves internal whitespace. It returns at most one current, public
profile on the first page, with a percent-encoded local link. Incomplete or
unavailable source or privacy evidence returns 503. Hidden and unknown
characters produce an empty result.

Tests use isolated schemas in `PORTAL_TEST_DATABASE_URL`, printing a skip notice
when unset. Run the optional browser test with Chromium and external
`PORTAL_BROWSER_MODULES` containing `playwright-core` and `@axe-core/playwright`:

```
cargo test --locked -p oteryn-web search::tests::postgres_browser_search_keyboard_and_accessibility -- --ignored --nocapture
```

It performs real keyboard searches in both languages at 360 and 1280 pixels,
checks escaped titles/no horizontal overflow and runs the WCAG 2.2 AA axe rules.
