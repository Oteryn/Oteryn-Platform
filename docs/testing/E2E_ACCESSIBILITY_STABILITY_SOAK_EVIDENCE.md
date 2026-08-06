# E2E Accessibility, Stability and Soak Evidence

## Scope

This record covers issue #110 / PR #111 and the bounded hardening slices added after the existing exact-SHA production-like acceptance baseline:

- P1 keyboard/focus accessibility interaction;
- P2 repeated-run flakiness measurement;
- P2 read-only public soak measurement.

Issue #114 owns the first scheduled runtime evidence recorded below. All evidence in this document is repository or controlled production-like staging evidence. It does not establish `PRODUCTION_PROVEN` behavior and does not change the independent Production Go-Live Gate in issue #91.

## Accessibility interaction profile

Project: `accessibility-chromium`.

Execution policy:

- Chromium desktop only;
- zero retries inside the spec;
- included in direct pull-request `critical` acceptance and in `full` acceptance;
- raw trace, automatic screenshot and video remain disabled;
- diagnostics contain exact tested SHA plus bounded console/page/request/server-error metadata and do not persist form values, cookies, MFA secrets or recovery codes.

Representative interaction evidence:

- login fields and submit are reached by repeated keyboard `Tab`, expose `:focus-visible` plus a visible computed focus indicator, and submit with `Enter`;
- password-recovery email and submit are reached and activated by keyboard;
- Account Overview `Create a character` link is reached with `Tab` and activated with `Enter`;
- character name/vocation/sex/submit controls are reached in keyboard traversal, including reverse `Shift+Tab` verification;
- MFA challenge input and submit are reached and completed with keyboard input/`Enter`;
- managed-page table `Edit` link is reached and activated with keyboard, followed by keyboard traversal through the edit form to `Save` and reverse focus verification.

The profile does not claim screen-reader compatibility from DOM/focus assertions alone.

First successful exact-head implementation evidence:

- exact tested SHA: `3bd1e4901a71841bc4593ec7e4efb98866c8c30f`;
- Acceptance E2E and Visual UX run: `29853941922`, attempt 1;
- profile: `critical`;
- aggregate result: `AUTOMATED_E2E_CRITICAL_PASS`;
- smoke: PASS, 10 s wall-clock;
- portability: PASS, 32 s wall-clock;
- responsive: PASS, 9 s wall-clock;
- resilience: PASS, 3 s wall-clock;
- accessibility: PASS, 6 s wall-clock;
- accessibility JUnit: 3 tests, 0 failures, 0 skipped, 5.392079 s total test time;
- artifact: `acceptance-e2e-critical-29853941922-1-direct`, digest `sha256:df7776df2c3ecb6e3199baab24469e351b3bbef4d099d916095a99f68f183b9a`.

During implementation an earlier WebKit portability run exposed an existing navigation race: the portability scenario began privileged navigation before the MFA-completion redirect had settled. The fix added an explicit post-MFA URL synchronization and the next exact-head portability run passed. The first accessibility run then exposed the same class of race in the new keyboard login helper; `keyboardLogin` now waits until navigation leaves `/login` before the caller begins the next route. Neither failure was masked with retries.

## Repeated-run stability profile

Workflow: `.github/workflows/acceptance-stability.yml`.

Policy:

- scheduled weekly and manually dispatchable;
- three fresh isolated jobs per run;
- each job calls the exact-SHA reusable acceptance workflow with profile `critical`;
- each iteration has a distinct `run_suffix` and artifact identity;
- `ACCEPTANCE_ZERO_RETRIES=1` forces Playwright global retries to zero for stability measurement;
- the caller uses `fail-fast: false`, so later iterations still run after an earlier iteration failure and preserve evidence for instability classification;
- MariaDB, Redis, MailHog, Laravel runtime and file-backed cache/session state are fresh per matrix job rather than reused across iterations.

This profile measures whether the bounded required critical acceptance remains stable across independent executions. A failed iteration is not masked by Playwright retry success.

### First scheduled three-iteration evidence

The first completed scheduled run after PR #111 merge is runtime-proven:

- workflow run: `30243589211`, attempt 1;
- event: `schedule`;
- started: `2026-07-27T06:42:08Z`;
- exact tested SHA: `37eb31d60aa8a47914745cd326aff6b313851dd0`;
- aggregate conclusion: PASS;
- retries: zero in every iteration;
- execution identity: three distinct matrix jobs and three distinct `repeat-1`, `repeat-2`, `repeat-3` artifact identities.

| Iteration | Job | Result | Profile wall-clock evidence | JUnit evidence | Artifact | Digest |
|---|---:|---|---|---|---:|---|
| 1 | `89905727036` | PASS | smoke 11 s; portability 59 s; responsive 32 s; resilience 2 s; accessibility 10 s | 7 + 27 + 24 + 2 + 6 tests; 0 failures/errors/skips | `8644201125` / `acceptance-e2e-critical-30243589211-1-repeat-1` | `sha256:ce1542b0815fb006d15f22cd92a5977fe0d3c1df6aca85bc4589a37a6afe3968` |
| 2 | `89905726989` | PASS | smoke 10 s; portability 60 s; responsive 34 s; resilience 3 s; accessibility 10 s | 7 + 27 + 24 + 2 + 6 tests; 0 failures/errors/skips | `8644204136` / `acceptance-e2e-critical-30243589211-1-repeat-2` | `sha256:1ada3ea282e9c77644824d424010004bdc378fc0b0ead262f83e8185f78168fd` |
| 3 | `89905727019` | PASS | smoke 10 s; portability 60 s; responsive 31 s; resilience 3 s; accessibility 9 s | 7 + 27 + 24 + 2 + 6 tests; 0 failures/errors/skips | `8644207634` / `acceptance-e2e-critical-30243589211-1-repeat-3` | `sha256:e98ab0296a5fc3a2b87467a1c6270a29d7819be0d8965747568e3cee184aa0cb` |

The first scheduled run therefore proves three successful independent zero-retry executions of the bounded critical profile on one exact SHA. It is a stability sample, not a universal flakiness guarantee and not a blocking threshold.

### Subsequent scheduled failure classification

Scheduled run `30790638508` on exact SHA `f2de161edc54ccb276b33d5901e03385c7d88c62` produced a meaningful later instability signal:

- iterations 1 and 2 passed in jobs `91613214517` and `91613214565`;
- iteration 3 failed in job `91613214607`;
- first failing profile: responsive;
- first failing project/test: `responsive-mobile` / `admin-wiki-administration.spec.mjs`;
- Playwright retries: zero;
- failure artifact: `8847001250` / `acceptance-e2e-critical-30790638508-1-repeat-3`;
- artifact digest: `sha256:f4cf18fcbf59065fb82437070935770e76f5b40038777a81d6d1ab9e99409db7`.

The failed assertion waited for the transient success flash `Wiki article submitted for review.` after the submit-for-review mutation. The same artifact's page-state evidence already showed durable `Status: In Review` plus the expected `Return to draft`, `Publish` and `Archive` actions. Setup, exact-SHA runtime, smoke and portability had passed. The failure is therefore classified as a **harness race around transient flash observation**, not a product lifecycle or infrastructure failure.

Current main waits for authenticated thumbnail activity to quiesce and verifies the durable `In Review` state and available next actions instead of treating the transient flash as the lifecycle source of truth. PR #495 subsequently merged deep exact-SHA validation evidence from run `30897646594`, artifact `8888425228`, digest `sha256:232e7ca9c3b5209f06ab850d8beb88cd429ce1d7fd8ef2d86b3ba2519242ad54`: responsive completed 42 tests with zero failures and the aggregate 630-test execution reported zero failures, errors, skips and retries.

This classification does not erase the failed run. It preserves the instability signal, the exact first failure and the verified remediation evidence without rerunning or retry-masking the original iteration.

## Read-only public soak profile

Project: `soak-chromium`.

Workflow: `.github/workflows/acceptance-soak.yml`.

Policy:

- scheduled weekly and manually dispatchable;
- default bounded duration: 300 seconds;
- zero retries;
- public read-only routes only: home, online, highscores and servers;
- no authentication, MFA, password recovery, account mutation, character mutation or privileged mutation in the soak loop;
- every navigation requires HTTP 200 plus a representative expected UI assertion.

Collected non-secret metrics:

- exact tested SHA;
- target and measured soak duration;
- iteration/request count;
- overall min/p50/p95/max navigation time;
- per-route request count and p50/p95/max navigation time;
- Laravel serve process-tree RSS start/end/max samples;
- Redis key count before and after the soak.

No latency, memory or Redis-key budget is enforced in the initial profile. Metrics are calibration evidence only until repeated runs establish normal variance and a defensible regression threshold.

### First scheduled soak evidence

The first completed scheduled run after PR #111 merge is runtime-proven:

- workflow run: `29987560312`, attempt 1;
- event: `schedule`;
- started: `2026-07-23T07:13:00Z`;
- exact tested SHA: `8006534108d835474dadd208b0ec934e4a12528b`;
- job: `89142739953`, `public-soak / acceptance`;
- conclusion: PASS;
- profile/project: `soak` / `soak-chromium`;
- retries: zero;
- target duration: 300 seconds;
- browser loop duration: 300 seconds;
- measured profile duration: 303 seconds;
- iterations: 441;
- requests: 1,764;
- artifact: `8555768555` / `acceptance-e2e-soak-29987560312-1-soak`;
- artifact digest: `sha256:d3caa7c21f577616a1aacad45276ea21b1211d8727489c6c06d6ad9fc01cc7f4`.

Overall navigation time in milliseconds:

| Minimum | p50 | p95 | Maximum |
|---:|---:|---:|---:|
| 23.023 | 97.995 | 262.457 | 1,406.794 |

Per-route navigation time in milliseconds:

| Route | Requests | p50 | p95 | Maximum |
|---|---:|---:|---:|---:|
| `/` | 441 | 177.640 | 311.512 | 450.357 |
| `/highscores` | 441 | 85.521 | 143.162 | 189.705 |
| `/online` | 441 | 106.657 | 151.386 | 1,406.794 |
| `/servers` | 441 | 86.677 | 139.920 | 235.402 |

Controlled-runtime resource calibration:

| Signal | Start | End | Maximum |
|---|---:|---:|---:|
| Laravel serve process-tree RSS | 181,476 KiB | 181,980 KiB | 183,424 KiB |
| Redis key count | 1 | 1 | not applicable |

Every navigation and representative UI assertion passed. Redis key count was unchanged. These values are the first non-secret isolated calibration baseline only; no latency, RSS or Redis-key threshold is inferred from this single run.

## Current implementation classification

- accessibility interaction mechanism and bounded required profile: `STAGING_PROVEN` on the exact SHA/run above;
- repeated-run workflow mechanism: `REPO_PROVEN`;
- first scheduled three-iteration runtime sample: `STAGING_PROVEN` for the controlled exact-SHA run `30243589211` only;
- later scheduled failure: retained and classified as a zero-retry harness race with subsequent exact-SHA remediation evidence;
- soak workflow/profile mechanism: `REPO_PROVEN`;
- first scheduled soak runtime sample: `STAGING_PROVEN` for the controlled exact-SHA run `29987560312` only;
- production behavior: `UNKNOWN` until directly verified where applicable.

## Production boundary

This work does not prove:

- final production keyboard behavior under the deployed edge/runtime stack;
- production browser/device assistive-technology compatibility;
- production long-duration memory stability;
- production latency distributions or performance budgets;
- production Redis/session/cache accumulation;
- production HA/failover behavior.

Those remain direct-production concerns where applicable and cannot be promoted from controlled staging evidence.