---
task_id: OTERYN-20261015-char-cmd-deploy-wiring
governing_issue: 1474
required_reads:
  - deploy/synology/README.md
search_first:
  - deploy/synology/scripts/lib.sh
optional_reads:
  - config/character-authority.php
---

# OTERYN-20261015-char-cmd-deploy-wiring

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

This task does the Synology deploy wiring that the Character Authority commands enablement runbook (`docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md`, PR #1632) lists as preconditions 2 and 3, so that web Character creation can be switched on by a reviewed `native-defaults.env` change. It switches nothing on.

## Acceptance criteria

- [x] `deploy/synology/nginx/internal-mtls.conf` exposes `character-commands/pending`, `character-commands/read` and `character-commands/receipt` on the internal mTLS listener.
- [x] `deploy/synology/compose.yml` maps `CHARACTER_AUTHORITY_COMMANDS_ENABLED`, `_COMMAND_IDENTITIES`, `_COMMAND_SOURCE_AUTHORITY`, `_COMMAND_MAX_PENDING_INTENTS` and `CHARACTER_AUTHORITY_WEB_CREATE_ENABLED` into the `platform` service, with fail-closed defaults (switches off, no identity, no source authority, bound 40).
- [x] `lib.sh` `oteryn_resolve_game_node_route_settings` resolves them from the environment file or `native-defaults.env` (`OTERYN_STAGING_NATIVE_CHARACTER_*`) and refuses a non-boolean switch, a bound outside 1..256, enabled commands without identities or source authority, and web creation without the commands.
- [x] CP decision 2026-10-15: the enablement bound is `CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS=40` in the deploy defaults and templates; the Platform code default stays 256.
- [x] Env examples (`.env.example`, `deploy/synology/.env.example`), `native-defaults.env` and the Synology README agree.
- [x] Production gate: not lifted. `CharacterCommandSettings::current()` honours the commands only in `testing` and `preproduction`. Game contract §14 step 4 names only a `testing` stack and then `preproduction`; it has no production step, and U-CC5 (production PKI and host list) is open. The Synology stack runs `APP_ENV=preproduction`, so the gate does not block it.

## Ownership

```yaml
owned_paths:
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/compose.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/native-defaults.env
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - .env.example
  - docs/agents/tasks/archive/OTERYN-20261015-char-cmd-deploy-wiring.md
modules:
  - deploy/synology
dependencies:
  - Game contract oteryn-game-character-authority-commands-v1 (GAME-CHAR-CMD-0)
  - PR #1632 (U-CC9 and the enablement runbook)
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game OTV2-20261015-char-cmd-node-template (commented [platform.character_commands] example)
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Validate with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T12:00:00Z
head: the head reported to the control plane
branch: claude/cc-deploy-enable-20261015
pr: 1633
status: completed
context_routes:
  - native-characters
  - execution-resources
owned_paths:
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/compose.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/native-defaults.env
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - .env.example
  - docs/agents/tasks/archive/OTERYN-20261015-char-cmd-deploy-wiring.md
proven:
  - The deploy workflow renders deploy/synology/.env from a fixed key list, so the command settings reach Platform only through lib.sh and native-defaults.env, as the bootstrap-intent settings do.
  - CharacterCommandSettings::current() returns null unless APP_ENV is testing or preproduction; the Synology workflow renders APP_ENV=preproduction.
  - docker compose config renders the five variables; nginx -t accepts the listener config.
derived:
  - Enabling is a reviewed native-defaults.env change; no NAS environment edit can carry these settings.
unknown:
  - The subject DN of the Game character-commands client certificate (not issued yet).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Lifting the production environment gate; rejected because Game contract §14 step 4 rolls out only to testing and preproduction and U-CC5 (production PKI) is open.
  - Passing the settings through the NAS environment file; rejected because the workflow rewrites that file from a fixed key list on every deploy.
changed_paths:
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/compose.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/native-defaults.env
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - .env.example
  - docs/agents/tasks/archive/OTERYN-20261015-char-cmd-deploy-wiring.md
validation:
  - command: pytest -q deploy/synology/tests/test_platform_mtls_contract.py
    result: PASS
    evidence: 21 passed, including the new resolver refusal test
  - command: bash -n deploy/synology/scripts/lib.sh; docker compose -f deploy/synology/compose.yml config; nginx -t on internal-mtls.conf with throwaway certificates
    result: PASS
    evidence: syntax OK; the five variables render with their defaults
  - command: python3 tests/ci/test_synology_*.py and deploy/synology/tests/test_fresh_baseline_contract.py
    result: PASS
    evidence: Synology deployment contract suites pass
blockers:
  - none
next_action: none. Review and merge PR 1633 after PR 1632; the owner then issues the certificate and flips native-defaults.env per the runbook.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository squash merge of PR 1633
source_branch_evidence: PR 1633 opened from claude/cc-deploy-enable-20261015 against main; the ref is deleted automatically on merge
```

## Notes

- This PR does not touch the files of PR #1629, #1631 or #1632. The runbook on #1632 still states the bound as an open question; the CP decision (40) needs a one-line edit there or a follow-up after it merges.
- `marketplace-scheduler` does not get the command settings; Bazaar transfers from the scheduler stay "commands unavailable" until a Bazaar enablement packet.
