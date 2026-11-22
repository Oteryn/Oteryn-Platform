# Daily puzzles

T93 implements §4 and §5.10 of the portal plan. No existing puzzle route or
puzzle wire contract was found in the Laravel routes or `docs/contracts`.
The feature does not consume or produce a game API message.

`OTERYN_DAILY_PUZZLES_ENABLED=true` mounts `/pl/puzzles` and `/en/puzzles`,
their `/{id}/image` reads and `/{id}/answer` form submissions. The flag is
false by default and accepts only the exact string `true`. Public world reads
also require `OTERYN_WORLD_ENABLED=true`. Configure `OTERYN_PUBLIC_ORIGIN`
for browser Origin verification. Authentication uses the existing browser
session; only active, verified accounts may read or answer puzzles.

## Publish a UTC day

Run the admin CLI from a trusted operator shell, with the portal database
configured and both `OTERYN_DAILY_PUZZLES_ENABLED=true` and
`OTERYN_WORLD_ENABLED=true`. Pass the UUID of an active, email-verified staff account with the
`puzzles.publish` permission, granted to the built-in `platform_admin` role.

```sh
oteryn-portal admin-cli puzzles publish publication.json --actor <account-uuid>
```

The JSON file is limited to 64 KiB. It contains a non-nil UUID
`publication_id`, a UTC `day` in `YYYY-MM-DD` format, and a `puzzles` array
with one to three entries and at most one entry per kind. Each entry supplies
the existing `definition` and, for image puzzles, a `media` reference. The
following quiz uses an already active and approved catalogue.

```json
{
  "publication_id": "019a0000-0000-7000-8000-000000000023",
  "day": "2099-01-01",
  "puzzles": [{
    "definition": {
      "kind": "quiz",
      "world_version": "019a0000-0000-7000-8000-000000000022",
      "question": {"pl": "Które zwierzę ma spiczaste uszy?", "en": "Which animal has pointed ears?"},
      "clue": {"pl": "Poszukaj małego leśnego zwierzęcia.", "en": "Look for a small woodland animal."},
      "choices": [
        {"label": {"pl": "Lis", "en": "Fox"}, "source": {"kind": "Creature", "key": "fox"}},
        {"label": {"pl": "Wilk", "en": "Wolf"}, "source": {"kind": "Creature", "key": "wolf"}}
      ],
      "correct_choice": 0
    }
  }]
}
```

`correct_choice` is zero-based. Every choice must reference an allow-listed
entity in the same active world version. Editorial staff verify the questions,
bilingual clues and labels, correct entity, and its correspondence to any image.

For monster media, use `{"kind":"monster","manifest":"sprites/manifest.json","request":{"kind":"creature","appearance_id":1,"pattern":null}}`
with the sprite pipeline's immutable manifest. The portal verifies the thumbnail
and stores a silhouette. For Atlas media, use
`{"kind":"atlas","manifest":"atlas/manifest.json","tile":"0/0/0.webp"}`
with the immutable public tile pipeline manifest for the supplied world version's
source commit.
Relative manifest paths resolve next to the publication JSON. Asset paths resolve
inside the manifest directory and the portal checks their digests against that
manifest. Quiz entries omit media. The portal fails closed if an asset or approved
source is unavailable.

New publications activate all supplied slots and append one `puzzles.publish`
audit entry in the same transaction. Future dates are scheduled for their UTC
day. Repeating the same document and ID returns `replayed: true` without
reopening its assets. Reusing an ID with changed content, publishing over an
occupied date and kind, or adding a slot from a different catalogue version
to an occupied UTC date returns `Conflict`. Published definitions are immutable.

Each account gets one attempt per puzzle. Repeating the same choice returns
the stored result; changing it returns a conflict. All three correct answers
on the same UTC date complete a day. Consecutive complete days increase the
streak; a gap resets the displayed current streak. Best streak and badges
at 3, 7 and 30 days persist. Badges cannot be transferred, purchased, redeemed
or sent to the game. There are no currencies or game rewards.

The server never returns the correct-choice index or entity keys. Database
uniqueness and a per-account row lock serialize answers and completion.
Source/account authorization precedes the transaction, using public crate
APIs without retaining a connection needed by those APIs. UTC expiry is
checked after acquiring the lock and before committing. Published definitions
cannot be updated or deleted; a revoked source fails closed rather than
silently changing a puzzle after players have answered it.

Tests use synthetic data only. PostgreSQL tests read
`PORTAL_TEST_DATABASE_URL`, print a skip notice when absent, and otherwise
create isolated schemas with the full migration chain.
