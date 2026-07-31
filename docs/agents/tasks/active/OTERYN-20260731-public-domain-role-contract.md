---
task_id: OTERYN-20260731-public-domain-role-contract
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - deploy/synology/README.md
search_first:
  - molehill.cloud
  - public endpoint
  - Game Gateway
optional_reads:
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
---

# OTERYN-20260731-public-domain-role-contract

## Goal

Persist the owner-designated public hostname roles for the Oteryn Platform web server and Oteryn Game Gateway so future agents do not confuse the two services or their origin ports.

## Acceptance criteria

- [x] `https://oteryn.molehill.cloud` is documented as the public Oteryn Platform website hostname.
- [x] `https://login.oteryn.molehill.cloud` is documented as the public Oteryn Game Gateway/login API hostname.
- [x] The Synology loopback origins `127.0.0.1:8000` and `127.0.0.1:8080` are recorded.
- [x] Canary legacy-login and game-protocol ports are explicitly distinguished from these HTTPS hostnames.
- [x] Agent navigation points to the durable endpoint contract.

## Ownership

```yaml
owned_paths:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/tasks/active/OTERYN-20260731-public-domain-role-contract.md
modules:
  - deployment
  - game-login
  - agent-governance
dependencies:
  - existing Synology runtime topology
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-31T21:33:00Z
head: 60765475642d912ab91953b2e7714bf95ab9e583
branch: docs/public-domain-role-contract-20260731
pr: 382
status: validating
context_routes:
  - agent-governance
  - architecture
  - canary-integration
owned_paths:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/tasks/active/OTERYN-20260731-public-domain-role-contract.md
proven:
  - The repository Synology topology binds Platform to 127.0.0.1:8000 and Game Gateway to 127.0.0.1:8080.
  - The repository owner designated https://oteryn.molehill.cloud for the website and https://login.oteryn.molehill.cloud for Game Gateway/login API.
  - PR 382 contains exactly four authorized documentation paths.
derived:
  - A durable public-endpoint contract and deployment note are the least ambiguous agent-facing sources of truth.
unknown:
  - Independent external reachability of the login hostname is not claimed by this documentation task.
  - Required CI conclusions for the latest PR head are not yet recorded.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Port 3031 belongs to Oteryn Platform.
  - The login hostname is the public website.
changed_paths:
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/tasks/active/OTERYN-20260731-public-domain-role-contract.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
validation:
  - command: compare main...docs/public-domain-role-contract-20260731
    result: PASS
    evidence: branch is four commits ahead with exactly four authorized documentation paths
  - command: repository documentation review
    result: PASS
    evidence: hostname roles match the owner instruction and existing Synology runtime topology
blockers:
  - none
next_action: Inspect PR 382 exact-head CI and keep the PR draft until required checks pass.
```

## Notes

This task records service identity and routing intent only. It does not claim production verification, modify Cloudflare, expose Canary TCP ports, or change application runtime behavior.
