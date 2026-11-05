# Atlas spawn overlay (T84)

Enable only with `OTERYN_ATLAS_SPAWNS_ENABLED=true`, `OTERYN_ATLAS_ENABLED=true`
and `OTERYN_WORLD_ENABLED=true`. All default to disabled. Browser mutations also
require the canonical `OTERYN_PUBLIC_ORIGIN` used by the shared session/CSRF layer.
The T41 viewer still requires its private tile directory.

The read-only Game reference is pinned at
`857f15f376907c592bb28d854fccff18a15cdf87`. Import `Spawn.Source` records as
`spawns` and creature definitions as `creatures` through the public world API.
The native declaration shape, rather than historical catalog/export relations,
governs these records (§6.1). Both source and target must be publishable and
allow-listed. No Laravel Atlas routes or map-annotation endpoints exist to reuse;
Laravel support ownership and pending-report limits inform the local seam.

`GET /atlas/spawns` requires `version` (full source commit), `floor` and inclusive
`min_x,min_y,max_x,max_y`. It returns one `{spawn,cell,creature,respawn_ms}` per
visible monster point. Missing or mismatched world state is unavailable, malformed
queries are 400. The viewer groups points in a screen-space grid when zoomed out,
counts distinct source spawns separately from monsters, and exposes floor,
coordinates, respawn seconds and creature links in a keyboard-accessible list.
No creature artwork is invented when a thumbnail-serving API is unavailable;
the numbered map symbol identifies points and groups.

Private routes require an active account and the shared synchronizer CSRF token:

- `GET /atlas/private-markers`: own markers and CSRF token only; private/no-store.
- `POST /atlas/private-markers`: `{x,y,floor,text}`, label 1–80 characters, 201;
  maximum 100 markers per account, checked under an account row lock.
- `DELETE /atlas/private-markers/{id}`: owner-scoped, 204 or 404.
- `POST /atlas/map-reports`: same location shape, description 10–2000 characters,
  201 after durable insertion into `atlas_map_reports`.

Coordinates are unsigned 16-bit, floor unsigned 8-bit. Unknown fields and invalid
numbers are rejected; surrounding whitespace and control characters in text are
422. Unauthenticated reads are 401, inactive accounts 403, foreign marker IDs 404.
No account ID is accepted from the client. All responses are private/no-store.

T33c is absent from the base and has no published dependency branch. Its bounded
seam is the local `atlas_map_reports` pending queue. Five pending reports or five
submissions in an hour refuse further submissions with 429; five rejected reports
refuse with 403. All limits use the same account lock. Future support integration
must drain this queue and supply its authoritative cross-category eligibility
policy through a public API; no other crate's tables or implementation are read.
There is no moderation UI in this task and no game mutation.

Validation includes PostgreSQL-backed ownership, suspension, expiry, CSRF,
visibility, floor, bad-input, report-limit and version checks. Optional browser
qualification uses `PORTAL_ATLAS_BROWSER_NODE`, `PORTAL_ATLAS_PLAYWRIGHT_MODULE`
and `PORTAL_ATLAS_CHROMIUM` to run `spawns_browser.cjs` against real handlers and
the same isolated PostgreSQL fixture. It checks PL/EN at 360 px, cluster separation,
counts, location links, keyboard panning, marker persistence/deletion and refusal.
