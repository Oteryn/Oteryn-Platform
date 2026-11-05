# World calculators (T71)

Public SSR GET forms: `/pl/wiki/calculators`, `/en/wiki/calculators`.
`PORTAL_CALCULATORS_ENABLED=true` enables them; every other value returns 404.
There are no writes, JavaScript requirements, account reads or currency transfers.
Invalid inputs return localized 400 responses. Missing, disabled, hidden or
malformed world policies return localized 503 responses, without an estimate.

World-dependent estimates additionally require `PORTAL_WORLD_ENABLED=true` and
explicit references to allow-listed records in the active T14 version:

- `PORTAL_CALCULATORS_EXPERIENCE_SOURCE=kind|key`: the complete finite
  `OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1` document, not a guessed cubic curve.
- `PORTAL_CALCULATORS_TRAINING_SOURCE=kind|key`: the training rates document
  (`schema_version=1`, enabled), including `exercise_gains`, `percent_scale`,
  and the selected dummy's integer percentage in `dummies`.

The server chooses these references; users cannot select arbitrary world records.
Results show the world version returned by the single allow-listed read.
Training estimates start from remaining mana/tries reported by the game, rather
than inferring missing skill thresholds or character progress. They count active
exercise time and charges, excluding interruptions and startup exhaustion.
Loot splitting takes net proceeds in whole units of one user-selected currency;
the remainder is explicitly unassigned. It does not invent NPC prices.

## Source and integration dependency

SOURCE: read-only Oteryn/Oteryn-Game at
`857f15f376907c592bb28d854fccff18a15cdf87`:

- `rulesets/character/experience/experience-table.json`: finite experience table.
- `rulesets/training/crystal-201189f-v1.json`: training gains and dummy percentages.
- `crates/gameplay/progression/src/loop_training.rs`: gains truncate per charge.
- `crates/gameplay/dispatch/src/gameplay_transport/loop_training/exercise.rs`:
  two-second exercise cadence.
- `crates/gameplay/characters/src/stamina.rs`: implemented OFFLINE-0 regeneration
  with ten-minute delay, 180/360-second periods, 2340/2520-minute boundaries,
  and elapsed time capped at 21 days before applying the delay.

The pinned stamina ruleset directory is explicitly `READY_UNPOPULATED`.
Stamina therefore uses the implemented formula; no populated world value is
replaced by a constant. Other estimates require their actual world records.
T14 currently imports only `content/**`, while experience and training rates are
under `rulesets/**`. This task does not widen that import contract or copy these
policies into Platform. The coordinator must arrange an approved T14 import path
before these two world-dependent forms can produce production estimates.

No calculator-specific governing contract or Laravel route was found in Platform.
Small test excerpts carry SOURCE notes; their remaining table rows are synthetic.
Run DB tests with `PORTAL_TEST_DATABASE_URL` set; unset URLs print a skip notice.

## Verification

The T71 checks pass with PostgreSQL 18.6 and `SQLX_OFFLINE=true`: eight focused
tests, including three PostgreSQL tests. The required workspace formatting,
Clippy and test commands also pass. Chromium at 360 px in both languages passed
keyboard-only form submission with JavaScript disabled, result and refusal
rendering, and horizontal overflow checks. Automated WCAG 2.2 AA checks found
zero violations on either localized form page; this is not a full accessibility
audit.

Full-binary smoke testing on the supplied base is blocked by its existing duplicate
`GET /online` registration between `community::online` and `web::redirects`.
Calculator browser checks therefore used an isolated local router harness.
The duplicate registration must be reconciled by the owning tasks/coordinator;
T71 does not modify their files or existing registrations.
