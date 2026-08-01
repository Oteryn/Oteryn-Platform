# Research purpose and safety scope

## Task policy and completion decision

```yaml
policy_version: 2
task_kind: audit
implementation_authorized: false
external_service_validation_authorized: false
context_pressure: medium
decomposition_decision: phased
execution_mode: chat
```

This document defines the durable authorization, safety and completion boundary for task `OTERYN-20260727-tibia-linux-runner-analysis` and PR #218.

The repository-side objective is a bounded, text-only interoperability audit of the official Tibia Linux client. The audit is complete when the static evidence, uncertainty classification, Oteryn/Canary relevance, validation limits and cleanup state are recorded coherently. Official-server acceptance and BattlEye enforcement remain `UNKNOWN` / `NOT_RUN`; they are not an acceptance criterion for this audit and must not keep its worker session or PR open merely to wait for credentials, authorization or a future live test.

Any official-service validation is a separate task. It requires explicit owner authorization, independently declared risk acceptance and a fresh task/branch/PR boundary before execution.

## Project purpose

The Tibia Linux protocol and transport analysis exists only to support interoperability between the new Oteryn client and the project-owned Oteryn/Canary server stack.

The intended outcome is to reproduce the behavior required by our own server and client architecture, including:

- account-to-game entry;
- protected and unprotected route modeling;
- transport framing and typed message handling;
- session entry and disconnect behavior;
- server-to-client world state needed to render our own map, creatures, statistics, inventory and UI state;
- deterministic local fixtures, mocks and Canary integration tests.

The official client is external evidence. It is not a runtime dependency, distributable project asset or authority to reproduce third-party protected-service behavior beyond the minimum evidence needed for Oteryn/Canary compatibility.

## Authorized internal research

Within this project purpose, internal technical work may include:

- static decoding and classification of client/server packets;
- reconstruction of message framing, schemas and state machines;
- mapping messages into project-owned map, creature, statistics, inventory and UI models;
- controlled packet capture and replay against project-owned Canary instances or local mocks;
- interoperability comparison against externally observed behavior;
- private diagnostic tooling, fixtures and protocol tests needed to build and validate Oteryn/Canary;
- bounded extraction of text, metadata, hashes, disassembly and pseudocode from lawfully obtained binaries, without committing or redistributing those binaries.

These results are project-internal engineering evidence. They must not be published, redistributed or packaged as general-purpose tooling for third-party gameplay manipulation, unauthorized access, anti-cheat evasion or exploitation.

## Explicit non-goals and forbidden actions

This work does not authorize:

- cheating or obtaining an unfair gameplay advantage;
- gameplay automation against third-party services;
- access to accounts, sessions, characters or infrastructure without authorization;
- patching, disabling, hooking, impersonating or evading BattlEye or another anti-cheat mechanism;
- concealment of a modified client or circumvention of service-side controls;
- exploitation, degradation or interference with official game services;
- redistribution of proprietary CipSoft binaries, archives, assets, credentials, cookies, session data or reusable exploit/bypass material;
- use of staging or production secrets, services, ports, databases, volumes or networks outside the declared analysis boundary;
- implementation changes to Oteryn, Canary or OTClient under this audit task.

Where protected behavior is relevant, model the boundary, preserve unknowns and fail closed. An observed unprotected route, reachable socket or client-side branch is not by itself evidence of server acceptance, an anti-cheat bypass or a security vulnerability.

## Evidence model

Every material claim must be classified as one of:

- `PROVEN`: directly supported by the analyzed binary, deterministic tooling, repository source, a bounded local test or live Git/CI state;
- `DERIVED`: an explicit conclusion that follows from listed proven facts;
- `UNKNOWN`: not established by available evidence and never to be replaced with a guess;
- `CONFLICT`: authoritative evidence disagrees and requires resolution.

Static client evidence cannot prove server-side field requirements, session acceptance, enforcement policy or exploitability. Localhost mocks prove only the bounded behavior exercised by the mock. External-service behavior remains external evidence unless a separately authorized validation task records a minimal decisive observation.

Large logs, binary extracts, captures and full disassembly do not belong in prompts or task checkpoints. Store only approved text evidence or references, keep sensitive material private and access-controlled, and never place credentials or session secrets in GitHub, artifacts, logs, task records or PR comments.

## Validation boundary

Preferred validation targets, in order, are:

1. static analysis of the identified binary;
2. deterministic local fixtures and mocks;
3. project-owned Canary instances with synthetic or project-owned credentials;
4. read-only comparison with current Oteryn/OTClient/Canary contracts.

A localhost synthetic challenge/login/session dry-run validates the local control sequence only. It does not establish official protobuf compatibility, official-server acceptance, protected/unprotected route policy, BattlEye enforcement or a security defect.

No official account login or game-world connection is required to complete this audit.

## Separate future live-validation gate

A future official-service test may proceed only under a new task when all of the following are explicit:

- the repository owner authorizes the exact test and accepts account-sanction risk;
- only the researcher's own disposable account is used;
- the session key and credentials are obtained legitimately and kept outside Git, logs and artifacts;
- no binary, library or traffic is patched, hooked, altered, replayed deceptively or used to impersonate BattlEye;
- the test is limited to the first decisive rejection or acceptance result and stops immediately;
- no gameplay, automation, interaction with other players or persistence beyond the minimum observation occurs;
- the result is reported as evidence, not converted automatically into bypass implementation work.

Without those conditions, the result remains `NOT_RUN` and `UNKNOWN`. That is a valid terminal state for the present static-analysis audit.

## Completion boundary

The current task is repository-complete when:

- the analyzed client identity and evidence sources are recorded;
- protected/unprotected routing, socket flow, login-message structure and BattlEye callback boundaries are documented to the level supported by evidence;
- current OTClient interoperability is assessed without inventing unknown server behavior;
- `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT` and rejected hypotheses are separated;
- the final report, callback addendum and this safety scope are committed;
- temporary automatic workflows are removed;
- text-only validation evidence is recorded;
- no proprietary binaries, secrets, credentials, cookies or session material are present in GitHub;
- the task checkpoint and PR accurately describe the static audit as complete and any live test as separate and unauthorized by default.

A worker must stop and checkpoint if ownership conflicts, sensitive material appears, the requested activity crosses the forbidden boundary, or new authorization is required. It must not remain active to poll, wait for credentials or supervise an external observation window.

## Operating rule

Prefer project-owned Canary, local mocks and disposable local fixtures. Treat third-party live-service behavior as external evidence only, keep unknowns explicit, stop at the first decisive result, and route any future implementation into its own bounded task with explicit ownership and validation.

Any packet decoding, map reconstruction or UI-state work must remain compatibility work for Oteryn against Oteryn/Canary, not tooling for third-party gameplay manipulation. Keep sensitive captures, session material, proprietary artifacts and high-risk implementation details private and access-controlled.
