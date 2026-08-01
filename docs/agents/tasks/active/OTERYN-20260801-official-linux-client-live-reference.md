---
task_id: OTERYN-20260801-official-linux-client-live-reference
required_reads:
  - AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/archive/OTERYN-20260727-tibia-linux-runner-analysis.md
  - docs/agents/reports/OTERYN-20260727-research-purpose-and-safety-scope.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-protected-route-analysis.md
  - docs/agents/reports/OTERYN-20260727-tibia-linux-battleye-callback-addendum.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
search_first:
  - OTERYN-20260801-official-linux-client-live-reference
  - OTERYN-20260727-tibia-linux-runner-analysis
  - PR 218
optional_reads: []
---

# OTERYN-20260801-official-linux-client-live-reference

## Goal

Build and validate a safe, reproducible capability to launch the unmodified official Tibia Linux client, authenticate with an account owned and explicitly supplied by the repository owner through a non-logged secret channel, enter a bounded official-game session, collect redacted interoperability evidence, and convert that evidence into concrete OTClient/Oteryn/Canary compatibility requirements for the project-owned OTS.

The invariant is:

> A researcher can reproduce one owner-authorized official-client Linux session without modifying or bypassing the client or BattlEye, without exposing credentials or session material, and can produce a durable evidence package that identifies what must be implemented or changed in the project-owned OTS stack.

## Why this task exists

The archived task `OTERYN-20260727-tibia-linux-runner-analysis` completed a static audit only. It did not build or validate the owner-requested live-login and live-reference capability. This follow-up task restores that missing objective without rewriting merged history.

## Policy

```yaml
policy_version: 2
task_kind: e2e
implementation_authorized: true
external_service_validation_authorized: true
external_service_execution_ready: false
context_pressure: high
context_growth: expected
authorization_basis: repository owner explicitly authorized use of an account they own and will supply for this bounded research purpose on 2026-08-01
decomposition_decision: phased
decomposition_reason: one coherent capability must progress through environment discovery, safe harness implementation, owner-gated live validation, evidence analysis, and cross-repository handoff
execution_mode: codex
execution_reason: requires a repository checkout, Linux runtime inspection, scripts, GUI/process/network tooling, focused test loops, and bounded live-session reproduction
```

## Authorization conditions

Live execution is authorized only when all of the following are true:

- the account is owned by the repository owner and explicitly designated for this research session;
- credentials are supplied through an ephemeral local secret mechanism or manual entry on the research host, never in chat, Git, GitHub issue/PR text, workflow logs, command history, screenshots, artifacts or reports;
- the official client and BattlEye components remain unmodified;
- no patching, hooking, DLL/SO injection, process-memory modification, anti-cheat bypass, impersonation, protocol abuse or concealment is attempted;
- no third-party account, character, service or infrastructure is accessed;
- the live session follows a predeclared minimal observation script and stops after the required evidence is collected;
- raw sensitive evidence remains on a private encrypted research volume with bounded retention;
- only redacted text evidence and non-sensitive hashes/manifests may be committed or attached to GitHub.

General authorization is recorded, but live execution is not ready until the exact account/character, secret-delivery mechanism, research host and observation script are persisted without credentials.

## Non-goals

This task does not authorize:

- cheating, botting or autonomous gameplay on the official service;
- bypassing or weakening BattlEye or other service controls;
- decrypting or intercepting credentials or session secrets;
- redistributing the official client, game assets or proprietary binaries;
- mirroring proprietary assets into the OTS;
- interacting with other players beyond unavoidable passive presence;
- implementing broad OTClient, Canary or server changes before live evidence identifies a bounded requirement;
- claiming full official compatibility from one session.

## Phases

### Phase 1 — discover

Prove the smallest viable isolated Linux research environment.

Required output:

- exact host/VM/container boundary and CPU/GPU/display capabilities;
- whether the official client and BattlEye can launch in that environment without modification;
- required outbound endpoints and filesystem/runtime dependencies;
- evidence-retention and redaction design;
- exact secret-delivery method that does not expose credentials;
- a decision between a dedicated interactive Linux host and a self-hosted research runner.

Do not attempt an official login in this phase.

### Phase 2 — implement

Build the local research harness under owned paths.

The harness must:

- verify the exact client package/binary identity before launch;
- launch the unmodified official client in an isolated graphical Linux session;
- avoid shell tracing and redact environment/process arguments;
- accept credentials only through the approved ephemeral mechanism or pause for manual entry;
- capture bounded process, window, network-tuple, timing and local-state evidence without memory injection or traffic modification;
- write raw evidence only to the private research volume;
- generate a redacted session manifest and evidence index suitable for GitHub;
- provide deterministic cleanup and prove no credentials/session tokens remain in temporary files, logs or process arguments;
- support a dry-run mode that exercises all local controls without contacting the official service.

### Phase 3 — owner-gated live validation

After the owner supplies the exact account/character and secure secret mechanism, run one bounded session using the official client.

Minimum observation script:

1. launch the verified official client;
2. authenticate with the owner-owned account;
3. reach the character list;
4. select the designated research character and enter the official game world;
5. observe initial login/world state while stationary;
6. perform only the explicitly approved minimal normal-client actions needed for evidence, preferably user-driven rather than automated;
7. log out cleanly;
8. stop capture, redact, verify cleanup and revoke/delete temporary secrets.

Stop immediately on unexpected anti-cheat warnings, account-security prompts, unexplained client modification warnings, scope expansion or any evidence of credential exposure.

### Phase 4 — analyze and hand off

Correlate the live session with the static reports and current project-owned implementations.

Produce:

- a state-transition timeline from launcher start through logout;
- a redacted endpoint/connection and message-boundary ledger to the extent observable without bypassing protections;
- official-client behavior observations for login, character selection, initial world entry, map/world state, creature/player state, statistics, inventory and UI events exercised by the script;
- `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT` classifications;
- a compatibility-gap matrix for `blakinio/otclient`, Oteryn Platform and Canary;
- separate bounded follow-up task proposals per repository/ownership domain;
- no speculative implementation presented as proven protocol behavior.

## Owned paths

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md
  - tools/tibia-linux-reference/**
  - .github/workflows/tibia-linux-live-reference.yml
private_runtime_paths:
  - encrypted research volume outside Git checkout
read_only_cross_repository:
  - blakinio/otclient
  - project-owned Canary repository resolved from live state
```

The worker must verify no overlapping active task owns these paths before editing. Cross-repository source changes are forbidden in this task; create separate follow-up tasks after evidence is complete.

## Acceptance criteria

- [ ] A dedicated isolated Linux research environment is documented and proven capable of launching the exact unmodified official client.
- [ ] The exact client package/binary identity is verified before every launch.
- [ ] A dry-run harness completes without contacting the official service and without leaking synthetic secrets.
- [ ] Credential handling uses an approved ephemeral/manual mechanism and leaves no credential or session material in GitHub, logs, process arguments, command history or retained temporary files.
- [ ] Raw evidence is stored only on a private encrypted research volume with explicit retention and deletion controls.
- [ ] The owner-gated official-client session reaches the character list and designated game world, or the first decisive failure is captured and classified.
- [ ] No client/BattlEye modification, bypass, hooking, injection or traffic alteration occurs.
- [ ] A redacted live-observation report and session manifest are committed.
- [ ] Static and live evidence are reconciled using `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT`.
- [ ] A compatibility-gap matrix identifies concrete OTClient/Oteryn/Canary requirements.
- [ ] Separate follow-up tasks are proposed for independently owned implementation domains; this task does not silently implement them.
- [ ] Focused, component and final validation pass on the exact final head.
- [ ] The task is merged and archived with one concrete terminal result.

## Validation ladder

```text
Focused:
- shell/Python lint and unit tests for launcher, redaction and cleanup logic
- synthetic-secret leak scan across logs, manifests, process arguments and temporary paths
- deterministic dry-run using a fake client process and local mock endpoints

Component:
- isolated Linux graphical-session launch test with the exact official client but no authentication
- private-volume evidence write/read/delete test
- redacted-manifest schema validation

Heavy final gate:
- one owner-authorized official-client login/world-entry observation session
- exact-head repository checks
- post-session secret revocation/deletion and retained-evidence audit
```

A failed heavy run must be reduced to the first relevant failure before any retry. Do not exceed two official-service attempts in one worker session.

## Durable evidence rules

Persist only compact references in the checkpoint:

- client version, hashes and Build ID;
- research-host identity without secrets;
- session ID and timestamps;
- redacted endpoint/state timeline;
- evidence-file hashes and private storage locations;
- first relevant failure;
- proven/derived/unknown/conflict findings;
- cleanup verification;
- exactly one `next_action`.

Never persist credentials, authentication codes, cookies, session keys, account identifiers, character names unless the owner explicitly approves a redacted alias, raw packet payloads containing secrets, full proprietary binaries or asset dumps.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
phase: local-harness-readiness
session_id: codex-20260801-linux-harness-001
session_role: implementer
execution_mode: codex
execution_reason: bounded Linux environment discovery, multi-file harness implementation, and focused dry-run validation require a checkout and terminal
updated_at: 2026-08-01T12:18:00Z
lease_expires_at: 2026-08-01T13:03:00Z
head: 448bdf20d52a271524fd4be5ffe8af785b79db7c
branch: feat/OTERYN-20260801-official-linux-client-live-reference
pr: 391
status: validating
context_routes:
  - agent-governance
  - security
  - testing
  - ci-repair
context_pressure: high
context_growth: expected
context_score: 11
estimate_confidence: medium
decomposition_decision: phased
decomposition_reason: environment feasibility, harness implementation, live validation and analysis are sequential gates of one capability
validation_level: component
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 1
human_interruptions: 1
last_completed_step: reduced the first post-commit component failure to unchanged baseline synthetic fixtures and added a branch-diff regression repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md
  - tools/tibia-linux-reference/**
  - .github/workflows/tibia-linux-live-reference.yml
proven:
  - PR 391 is an open draft from the approved repository to main at head 03ee1a231d60e7b847980df15b81f0a9bee0ade0 and changes only the two authorized planning paths.
  - No other active task or open PR owns the declared harness, report, workflow, or checkpoint paths.
  - The first relevant CI failure is Agent Governance checkpoint-validation because first_failure was null instead of the required mapping.
  - The available Linux boundary is Ubuntu 26.04 x86-64 under WSL2 on Windows 11 Pro with WSLg DISPLAY and Wayland sockets and a visible GPU device.
  - Unprivileged user and network namespaces work through unshare -Urn in the available Ubuntu environment.
  - The WSL2 graphical fake-client component passed in a distinct loopback-only network namespace and retained only mode 0600 redacted JSON outside Git.
  - Eleven focused unit tests, Python compilation, manifest validation, workflow YAML parsing, checkpoint validation, tracked-file token scanning, and git diff checks pass.
  - origin/main advanced to ede1dfc44ae50da3e8d0b0b44d0fbe14f6c847dc only in non-overlapping Cloudflare, deployment, public-domain, and feature-test paths.
derived:
  - WSL2 is sufficient for deterministic synthetic harness validation but is not evidence that BattlEye supports virtualization.
  - A normal dedicated interactive Linux host remains the required fallback if the official client or BattlEye refuses WSL2 or virtualization.
  - external_service_execution_ready must remain false until encrypted storage, the exact approved package identity, and the no-authentication official component launch are proven.
unknown:
  - Whether the host C volume is encrypted; the current process cannot query BitLocker state and WSL2 cannot prove host-volume encryption.
  - The private official client package path and approved package SHA-256 are unavailable in this checkout.
  - Whether the exact official client and BattlEye start unmodified in WSL2.
conflicts: []
first_failure:
  marker: first post-commit synthetic component attempt reported tracked-files because generic token detection reclassified three unchanged repository test fixtures
  evidence: safe filename-only diagnostic identified one archived task and two existing game-gateway tests; exact synthetic run values were absent
rejected_hypotheses:
  - The current checkout was already the task repository: the initial workspace is blakinio/otclient, so a separate Oteryn-Platform checkout was created without modifying otclient.
  - The original checkpoint was valid: local and CI validation proved first_failure must be a mapping, and the checkpoint now passes contract v1.
  - A synthetic credential leaked during the post-commit component attempt: the minimized diagnostic proved only unchanged baseline token-shaped fixtures triggered generic scanning.
changed_paths:
  - .github/workflows/tibia-linux-live-reference.yml
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md
  - docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md
  - docs/agents/reports/OTERYN-20260801-official-linux-client-live-reference-plan.md
  - tools/tibia-linux-reference/**
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260801-official-linux-client-live-reference.md --require-checkpoint
    result: PASS
    evidence: one task checkpoint validated against contract v1
  - command: python3 -m compileall -q tools/tibia-linux-reference
    result: PASS
    evidence: all harness Python sources compiled under Ubuntu 26.04 Python 3.14.4
  - command: PYTHONPATH=tools/tibia-linux-reference python3 -m unittest discover -s tools/tibia-linux-reference/tests -v
    result: PASS
    evidence: 11 focused preflight, identity, redaction, manifest, event, leak-scan, and cleanup tests passed
  - command: python3 tools/tibia-linux-reference/run.py validate-manifest tools/tibia-linux-reference/examples/session-manifest.synthetic.json
    result: PASS
    evidence: redacted synthetic example passed schema v1 validation
  - command: WSL2/WSLg synthetic dry-run
    result: PASS
    evidence: fake X11 client mapped and destroyed, TEST-NET connection was denied in a distinct loopback-only namespace, 1308 files were scanned, and cleanup passed
  - command: official-mode preflight and exact identity component launch
    result: BLOCKED
    evidence: encryption cannot be proven and approved private package identity is unavailable; no official client or service was contacted
  - command: post-commit WSL2/WSLg synthetic dry-run at 448bdf20d52a271524fd4be5ffe8af785b79db7c
    result: FAIL
    evidence: first failure was tracked-files generic detection against three unchanged synthetic fixtures
  - command: focused branch-diff and retained-output secret-scan regression
    result: PASS
    evidence: exact values remain scanned across all tracked files while generic patterns fail only for the branch diff and new retained outputs
blockers:
  - A dedicated interactive Linux host with a provably encrypted private evidence volume and the owner-approved exact package identity is required for the official no-authentication component gate.
next_action: Restack the repaired branch onto current origin/main, then rerun the synthetic component gate before pushing PR 391.
```

## Final response contract

```text
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <compact result>
VALIDATION: <exact checks and outcomes>
DURABLE_STATE: <task path, branch, head, PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```
