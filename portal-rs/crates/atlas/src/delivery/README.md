# Delivery map

`/{pl,en}/atlas/delivery` is mounted by the binary. Both
`PORTAL_ATLAS_DELIVERY_ENABLED=1` and `PORTAL_WORLD_ENABLED=1` are required.
Other values, including absent configuration, deny access. No migration is needed.

Reads use the public `WorldStore` API and its active-version/publication/visibility
checks. Import mappings must expose the source families as `Item`, `Creature`,
`Loot`, `Spawn.Source`, and `Quest`. Original `definition` and `declaration`
record envelopes are supported. Mixed world versions are refused.

The reference is Oteryn/Oteryn-Game at
857f15f376907c592bb28d854fccff18a15cdf87. No Game fixtures are copied:
tests use synthetic records with the same small structural shapes. Quest recipe
stages and their `target_items` use explicit item keys only. Resolution markers
veto references, even when a key is present. Names are never resolved locally.

Creature loot references join visible loot entries to visible items. Spawn points
retain published coordinates and respawn times. Missing level/city metadata is
never inferred; restrictive filters exclude sources with unknown metadata.
Tiers are relative heuristic estimates using ppm chance, visible point count and
source level. Unknown level or absent spawns makes the tier unavailable.

`estimate` ports the deterministic algorithms and numeric regressions from
Oteryn/Oteryn-Atlas revision 0bbd188e8ba3e59290f4d9da754cda9e2418919c:
`src/browser/farm-intelligence.mjs`, `tests/farm-intelligence.mjs`, and
`docs/testing/ATLAS-FARM-ESTIMATOR-NUMERIC-CONTRACT.md`. It supports fixed
Bernoulli quantities and exact finite ppm PMFs. The discovery adapter uses fixed
quantities only: min/max bounds alone do not establish an exact quantity PMF.
Out-of-envelope, zero-progress and unsupported distributions never produce
approximate probability thresholds. All displayed results describe generated
drops, not personal acquisition or guaranteed loot.

T84 and T14b branches were absent at task start. `Spawn`/`Snapshot` are the narrow
spawn seam; `wiki_map_link` provides encoded item/creature entry links for wiki
entity templates when those pages arrive. Creature entry links restrict the item
selector to visible loot relations. Outgoing links use the plan's wiki and atlas
routes. No wiki implementation is replaced or duplicated here.

Tests use `PORTAL_TEST_DATABASE_URL`, with a printed skip notice when unset.
The three PostgreSQL tests create isolated schemas and exercise the HTTP success
path in both languages, hidden-source refusal, bad item input and disabled world
reads. An optional `PORTAL_DELIVERY_PREVIEW_DIR` exports the HTTP-rendered HTML
and stylesheet for browser inspection using synthetic test data.
