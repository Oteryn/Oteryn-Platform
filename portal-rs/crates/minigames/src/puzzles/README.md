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

A trusted editorial/worker caller publishes a `Definition` with
`Puzzles::publish(day, definition, media)`. This is a Rust service interface,
not a public publishing endpoint. The caller must have editorial publication
authority and verify that the question, bilingual clues, choice labels,
correct entity and selected image agree. No source family or appearance
mapping is guessed. All choices must resolve through the public `WorldStore`
API to allow-listed entities in the same active version. Missing or hidden
entities and inactive versions deny reads and submissions.

Publish one immutable definition per UTC date and kind: monster silhouette,
Atlas location, and world quiz. Future dates may be scheduled. Past dates
cannot be published or answered. `Media::monster` verifies a creature
thumbnail against the sprite pipeline's manifest and turns it into a light
silhouette before storing it. `Media::atlas` accepts an exact tile from the
public Atlas pipeline manifest, keyed by the world version UUID. Both verify
bytes and bound decoding, and neither publishes source filenames. When a
pipeline or source is unavailable, publication must remain unavailable.
The editorial caller supplies a meaningful equivalent clue in both languages.

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
