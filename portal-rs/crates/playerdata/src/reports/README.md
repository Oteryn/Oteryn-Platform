# Weekly reports and private annotations

`Reports` is the T82 service. `http::router` accepts an explicitly supplied service;
`http::configured_router` defaults to disabled and uses `DenySource`. No public
character projection, browser account ID or direct Game database can supply authority.
The binary mounts the configured router. `OTERYN_JOURNAL_REPORTS_ENABLED=true`
only enables the surface: it cannot enable the missing source adapter.

## Dependency integration

T73 was not present at implementation time. Its adapter must implement
`JournalSource` from accepted, private, complete journal projections. `snapshot`
must establish current account privacy, authoritative transfer cut, restore
fences, accepted retention and contiguous coverage of the requested interval,
including known-empty intervals. It must fence response publication at an
ownership/privacy change; a boolean copied from a historical row is insufficient.
`authorize` applies the same requirements to a session or character annotation.
Unknown source evidence denies access. A source outage is unavailable, not empty.
The default source denies every operation. No ingest wire is invented here;
G-9/G-10/G-11 ingestion belongs to the upstream private journal boundary.

T36 is used through its public `Notifications` API. A worker with a qualified
source can invoke `email_week` once a completed week is ready. The operation
requires both default-off flags, a report opt-in and the existing optional News
email preference. It deduplicates account/week in the same transaction as the
notification outbox. Mail contains a sign-in link only, never counts, drops or
notes. Report opt-out prevents new enqueues; already queued links remain governed
by T36's delivery-time account and News consent checks. An independent journal
mail category and scheduling registration require the notification owner.

T09 has no section-provider registry. `export_notes` is the composition seam for
its owner-scoped export; `weekly` provides retained session/drop projections.
The dedicated journal download routes work through the same service and source.
The coordinator must connect the section to T09 when an extension API exists;
this task does not edit identity implementation files.

## Conservative contract decisions

- The user-provided G-9 revision-2 interpretation applies: journal sessions run
  from game entry to durable logout; `character_revision` replaces the old field.
  Hunting areas and the 15-minute no-kill boundary describe segments, not sessions.
- Reports select sessions by durable end time in a completed UTC Monday-to-Monday
  week. A session is never split between reports. Drops are selected through those
  sessions, including occurrences before the week boundary. Drop quantities are
  not added to loot a second time. Incomplete sessions deny the whole report.
- G-9's proposed 90-day bound is stricter than the plan's 365-day detail retention;
  out-of-window requests deny. The adapter must apply record-level expiry, deletion
  and replay fences. An unknown transfer cut denies both possible owners.
- G-10 inventories/trades and G-11 progress are separate families and cannot be
  used to infer sessions, drops, current item possession or report completeness.
- Staff authority, public sharing, monetary valuation and game commands confer no
  report access. The service issues no game commands and makes no profit claims.
- Session totals use exact integers. JSON experience, quantities and character
  revisions are decimal strings. Zero hunting duration is valid and no rate is
  invented. CSV uses CRLF, quotes all cells and neutralizes formula-leading cells.

## Browser surface

Both `pl` and `en` expose `/account/journal/reports` with a required `week` Unix
second timestamp. `format=json` downloads the report; `format=csv` with
`dataset=sessions` or `dataset=drops` selects a CSV. Unknown formats deny.
`reports/notes?kind=session|character&id=<uuid7>` reads or updates a plain-text
annotation; POST requires the current session CSRF token, and empty text deletes.
`reports/notes/export` downloads authorized notes. `reports/email` updates the
report opt-in through CSRF-protected POST. Only an active verified account with a
current browser session can use the surface. All responses are private/no-store.
Foreign targets return 404 and malformed or excessive notes return 422.

The annotation tables cascade on account erasure. A journal adapter must suppress
expired/deleted targets immediately and coordinate annotation payload purging with
its retention job before activation. No upstream ingestion or mail scheduler is
activated by this task.
