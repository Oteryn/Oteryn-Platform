---
task_id: OTERYN-20260731-public-domain-role-contract
repository: blakinio/Oteryn-Platform
historical_pull_request: 382
historical_head: 2b295a170ba37bbbe1e7f7f4d711c14fed3fd26a
merge_commit: 4ba009ffd886d06c593ec3014b3219c2a887e9ab
completed_at: 2026-07-31T21:41:48Z
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
search_first:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
optional_reads: []
---

# OTERYN-20260731-public-domain-role-contract

## Goal

Persist the owner-designated public hostname roles for the Oteryn Platform web server and Oteryn Game Gateway so future agents do not confuse the services or their origin ports.

## Delivered bounded scope

- [x] `https://oteryn.molehill.cloud` is documented as the public Oteryn Platform website hostname.
- [x] `https://login.oteryn.molehill.cloud` is documented as the public Oteryn Game Gateway/login API hostname.
- [x] Synology loopback origins `127.0.0.1:8000` and `127.0.0.1:8080` are recorded.
- [x] Canary legacy-login and game-protocol ports are distinguished from the HTTPS hostnames.
- [x] Agent navigation points to the durable endpoint contract.

## Terminal ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260731-public-domain-role-contract.md
live_lease: none
live_claim: none
released_historical_paths:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/tasks/active/OTERYN-20260731-public-domain-role-contract.md
modules:
  - historical endpoint-role documentation evidence
dependencies: []
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T21:13:00Z
head: 2b295a170ba37bbbe1e7f7f4d711c14fed3fd26a
branch: docs/public-domain-role-contract-20260731
pr: 382
status: completed
context_routes:
  - architecture
  - deployment
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260731-public-domain-role-contract.md
proven:
  - Pull request 382 is closed and merged as 4ba009ffd886d06c593ec3014b3219c2a887e9ab from final head 2b295a170ba37bbbe1e7f7f4d711c14fed3fd26a.
  - The delivered change was documentation-only and modified exactly the endpoint contract, Synology endpoint note, repository map and historical active task record.
  - The historical source branch remains present but is classified as retained merged evidence with no live task, claim, lease, dependency or authority.
  - This archive is the sole durable historical task record and owns only its own archive path.
  - Canonical endpoint, deployment-note and repository-map ownership is released for future separately claimed work.
  - The documentation records service identity and routing intent only.
derived:
  - The bounded contract-writing task is terminal and does not own current endpoint, deployment or agent-routing documentation.
unknown:
  - Independent external reachability of either hostname.
  - Current Cloudflare routing, TLS, Access, WAF, origin health, staging state or production readiness.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Documented hostname naming proves public reachability or production readiness.
  - This historical task authorizes Cloudflare, runtime, staging, production or external-repository changes.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260731-public-domain-role-contract.md
  - docs/agents/tasks/archive/OTERYN-20260731-public-domain-role-contract.md
validation:
  - command: GitHub pull request 382 terminal-state verification
    result: PASS
    evidence: merged=true, final head 2b295a170ba37bbbe1e7f7f4d711c14fed3fd26a, merge commit 4ba009ffd886d06c593ec3014b3219c2a887e9ab
  - command: historical branch classification
    result: PASS
    evidence: docs/public-domain-role-contract-20260731 remains associated with merged PR 382 and has no live claim, task dependency or open PR.
blockers: []
next_action: none
```

## Nonclaims

This archive does not prove hostname reachability, Cloudflare correctness, TLS validity, origin health, staging readiness or production readiness. Those require separate tasks and direct environment evidence.