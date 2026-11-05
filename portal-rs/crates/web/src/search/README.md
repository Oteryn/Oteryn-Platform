# Portal search (T48)

`GET /search`, `/pl/search` and `/en/search` render the shared component system
with Polish/English Fluent catalogs. `q` is empty or 2–80 characters after
whitespace normalization; control characters are rejected. `page` is 1–1000.
Unknown parameters, types and malformed pagination answer 422. Database/source
failure answers 503. Empty searches do not enumerate documents. Search responses
are no-store/noindex. The existing locale/source WikiSearch limiter applies.

`kind` selects `news`, `pages`, `events`, `wiki`, `world` or `characters`.
Omitting it searches editorial sources together. Each page contains 20 results,
with an extra candidate to detect the next page; sorting has deterministic ties.
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

T11 and T14b implementation code is already integrated in main. T24b's remote
branch and public profile/search API were absent at task admission. Character
search therefore answers 503 and has no projection-table fallback. `Provider` is
the isolated integration seam: supply it only behind an off-by-default feature
gate, and require current accepted source freshness, lifecycle and Platform privacy
at every search. Never infer stable identity or visibility from a name. The
provider must return 21 rows for lookahead, enforce bounded full-text matching and
page limits, and percent-encode locale-prefixed local result destinations.

Tests use isolated schemas in `PORTAL_TEST_DATABASE_URL`, printing a skip notice
when unset. Run the optional browser test with Chromium and external
`PORTAL_BROWSER_MODULES` containing `playwright-core` and `@axe-core/playwright`:

```
cargo test --locked -p oteryn-web search::tests::postgres_browser_search_keyboard_and_accessibility -- --ignored --nocapture
```

It performs real keyboard searches in both languages at 360 and 1280 pixels,
checks escaped titles/no horizontal overflow and runs the WCAG 2.2 AA axe rules.
