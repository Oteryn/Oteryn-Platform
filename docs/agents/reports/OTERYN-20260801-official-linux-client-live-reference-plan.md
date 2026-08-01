# Official Tibia Linux client live-reference plan

## Purpose

This plan corrects the gap left by the completed static audit. The required capability is not merely to understand the Linux binary statically, but to run the unmodified official client in a controlled Linux environment, authenticate with the repository owner's account through a non-logged mechanism, enter a bounded official-game session, and turn the observed behavior into implementable compatibility requirements for the project-owned OTClient/Oteryn/Canary stack.

The live official service is a reference system only. The target of implementation is the project-owned OTS.

## Safety and authorization boundary

Authorized:

- launch the lawfully obtained, unmodified official Linux client;
- use only an account owned and explicitly designated by the repository owner;
- perform one bounded login/world-entry observation session and later owner-approved repetitions when a specific evidence gap requires them;
- collect local process, graphical, timing, filesystem and network evidence on the researcher's own host;
- compare observed behavior with project-owned source and services;
- build private research tooling and redacted interoperability fixtures.

Not authorized:

- modifying, patching, hooking or injecting into the official client or BattlEye;
- bypassing anti-cheat, route policy, authentication or server controls;
- automating gameplay, farming, combat, interaction with other players or persistence on the official service;
- extracting or redistributing proprietary client binaries or assets;
- publishing credentials, account identifiers, session tokens, raw sensitive payloads or reusable bypass material;
- treating TCP reachability or one accepted session as proof of complete protocol compatibility.

## Recommended environment

Use a dedicated Linux x86-64 research host or VM with an interactive graphical session and no access to Oteryn staging or production secrets.

Preferred properties:

- disposable OS image or snapshot;
- dedicated non-privileged user;
- encrypted evidence volume mounted outside the Git checkout;
- outbound network access limited to package dependencies and the official client endpoints required for the test;
- no inbound exposure;
- screen recording and packet capture disabled by default and enabled only for an approved session;
- automatic cleanup of temporary home, cache, environment and history files;
- a separate label such as `oteryn-research`, not the staging runner label, if GitHub Actions orchestration is used.

Whether BattlEye permits the chosen VM/headless setup is `UNKNOWN` until Phase 1. Do not attempt to work around a refusal. Move to a normal interactive Linux host if necessary.

## Credential handling

The repository owner must not paste the account password or authenticator code into ChatGPT, GitHub, a task file, a PR, an issue, workflow input, command line or shell history.

Acceptable mechanisms, in priority order:

1. manual credential entry directly into the official client on the research desktop;
2. a local interactive secret prompt whose value is passed only through a private pipe or protected temporary descriptor and is never echoed, logged or retained;
3. an owner-created secret in a dedicated protected local secret store, consumed only on the isolated host and deleted/revoked after the session.

Credentials must not be placed in process arguments or ordinary environment variables visible through process inspection. The harness must prove cleanup after dry-run and after live execution.

## Harness architecture

The implementation should remain small and inspectable.

### Preflight

Responsibilities:

- verify OS, architecture, display/session and required libraries;
- verify official package and executable hash/version/Build ID;
- verify the evidence volume is encrypted/private and outside Git;
- verify no tracing mode or command echo is enabled;
- verify the session ID and retention deadline;
- refuse to start if credentials appear in configuration, environment, arguments or tracked files;
- record a redacted preflight manifest.

### Launcher

Responsibilities:

- launch the exact unmodified client as a dedicated user;
- never alter, preload, wrap, inject or debug the client/BattlEye process;
- expose manual or protected interactive authentication;
- track process/window lifecycle and exit status;
- stop on unexpected modification/anti-cheat warnings;
- support a fake-client dry-run mode without official network access.

### Evidence collector

Allowed evidence:

- monotonic and wall-clock timestamps;
- process start/exit and window-state transitions;
- executable/library hashes and package metadata;
- redacted screenshots or screen recording when explicitly enabled;
- network endpoint tuples, connection timing, sizes and packet captures stored only on the private volume;
- local filesystem change inventory limited to the dedicated research profile;
- user-approved observation notes.

The collector must not decrypt, modify, replay or inject official traffic. Raw captures remain private. GitHub receives only a redacted timeline, hashes and high-level observations.

### Redactor and leak scanner

Before any output leaves the private volume, scan for:

- email/login/account identifiers;
- character names unless replaced by an approved alias;
- passwords and authenticator codes;
- cookies, session keys, authorization headers and token-like values;
- command lines and environment dumps;
- raw packet payloads;
- local host/user paths that identify the owner unnecessarily.

A redacted artifact is publishable only after the leak scan passes.

### Cleanup verifier

The verifier must confirm:

- client and capture processes stopped;
- protected temporary descriptors/files removed;
- shell/history files contain no secret material;
- temporary client profile handling matches the retention policy;
- raw evidence remains only in the declared private location;
- owner-created temporary secrets are deleted or revoked;
- Git working tree and GitHub-visible outputs contain no sensitive material.

## Observation script

The first live session is intentionally small:

1. start capture and record the verified client identity;
2. launch the official client;
3. authenticate using the approved mechanism;
4. observe account login and character-list presentation;
5. enter the designated research character;
6. remain stationary until initial world state stabilizes;
7. record the visible map viewport, player/creature state, statistics, inventory and UI state without asset extraction;
8. perform only separately approved normal-client actions required to distinguish a specific state transition, preferably manually;
9. log out through the normal client flow;
10. stop capture, redact, scan and clean up.

No combat, trading, chat, automation, repeated movement route, interaction with other players or long-running observation belongs in the first session.

## Evidence-to-OTS mapping

The live report must map observations into these independent domains:

| Domain | Live evidence target | Expected follow-up owner |
|---|---|---|
| Login service | account login, world/character selection, errors and transitions | Oteryn Platform / gateway |
| Game transport | route selection, connection lifecycle, framing boundaries observable without bypass | OTClient and Canary protocol owners |
| Initial world state | map viewport, player placement, creatures and known tiles | OTClient + Canary |
| Player state | health, mana, level, skills, conditions and statistics | OTClient + Canary |
| Inventory/equipment | visible slots, containers and item-state transitions | OTClient + Canary |
| UI state | messages, dialogs and client state changes relevant to protocol behavior | OTClient |
| Secondary connection | whether and when a second route is used, without defeating protection | OTClient + Canary |

For every row, separate:

- `PROVEN`: directly observed and timestamped;
- `DERIVED`: explicit inference from proven observations;
- `UNKNOWN`: not visible or not safely testable;
- `CONFLICT`: live evidence disagrees with static analysis or current project behavior.

## Deliverables

Expected repository outputs:

- `tools/tibia-linux-reference/README.md`;
- bounded launcher/preflight/collector/redactor/cleanup tooling;
- deterministic fake-client dry-run tests;
- `docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md`;
- redacted session manifest schema and example using synthetic values;
- compatibility-gap matrix;
- follow-up task proposals for each independently owned implementation domain.

Private-only outputs:

- official client package and binaries;
- raw packet capture;
- unredacted screenshots/video;
- detailed local runtime logs;
- any account/session identifiers;
- temporary client profile where retention is justified.

## Stop and escalation rules

Stop immediately when:

- the environment requires client/BattlEye modification to proceed;
- credentials or session material may have leaked;
- the official service shows an anti-cheat, account-security or unusual authorization warning;
- the requested observation would become gameplay automation or affect another player;
- the first decisive login failure is captured and a cheap local cause has not yet been isolated;
- ownership expands into OTClient or Canary source changes before the evidence package is complete;
- a material product, legal, account-risk or safety decision requires the owner.

The worker checkpoints and exits instead of waiting for credentials or keeping a live session open.

## Local-harness-readiness checkpoint (2026-08-01)

The compact Python harness, schema/example, unit suite and Linux workflow are implemented under the
declared owned paths. A WSL2/WSLg fake-client run proved the graphical lifecycle, distinct
loopback-only network namespace, denied reserved-address connection, exact synthetic-secret scan and
cleanup behavior without contacting the official service.

This phase remains fail-closed with `external_service_execution_ready: false` because the current
worker cannot prove host-volume encryption, the private official package and approved package hash
are unavailable, and the official client/BattlEye component launch has therefore not run. The next
phase must use a dedicated interactive Linux host with a provably encrypted private volume and the
owner-approved exact package identity; it must run only the no-authentication component gate before
any separate owner-gated live authorization.
