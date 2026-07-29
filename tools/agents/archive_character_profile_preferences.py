from __future__ import annotations

import re
from pathlib import Path

ACTIVE_TASK = Path('docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md')
ARCHIVE_TASK = Path('docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md')
ACTIVE_WORK = Path('docs/agents/ACTIVE_WORK.md')
PROJECT_STATE = Path('docs/agents/PROJECT_STATE.md')

FINAL_CHECKPOINT = '''## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T21:45:00Z
head: 3797a094cfa522f5147d624786f49fee5027c77b
branch: feat/OTERYN-20260729-character-profile-preferences
pr: 308
status: ready
context_routes:
  - agent-governance
  - architecture
  - auth-identity
  - canary-integration
  - database
  - security
  - web-cms
  - testing
owned_paths:
  - app/CharacterProfiles/**
  - app/Http/Controllers/CharacterProfiles/**
  - app/Http/Requests/CharacterProfiles/**
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - database/migrations/*character_profile_preferences*.php
  - routes/modules/character-profile-preferences.php
  - resources/views/identity/account/**
  - resources/views/game/character.blade.php
  - lang/{en,pl}/character_profiles.php
  - tests/Feature/CharacterProfiles/**
  - scripts/acceptance/**character-profile**
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - .github/workflows/community-data-acceptance.yml
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md
proven:
  - Platform-owned character profile preferences store a bounded escaped owner comment, per-character field visibility and an optional main-character selection without mutating Canary.
  - Every edit and update revalidates the ready immutable Identity-to-Canary binding and current active character ownership through the read-only Canary connection; foreign, stale and unavailable states fail closed.
  - Account-level association and status privacy remain upper bounds, hidden siblings are excluded from related-character output and no Platform or Canary internal identifiers are exposed.
  - Identity-row locking and the real-MariaDB race test leave at most one main character per Identity.
  - Exact final head 3797a094cfa522f5147d624786f49fee5027c77b passed all 11 required workflows: CI 30490007511, Agent Governance 30490007484, Portal Acceptance Contract 30490007458, Community Data Acceptance 30490007443, Phase 7 Production-Like Validation 30490007483, Platform DB Outage Validation 30490007507, Edge Security Emulation 30490007432, Game Auth Ticket Concurrency 30490007493, Acceptance E2E and Visual UX 30490007509, Synology Production Target Preflight 30490007537 and Build Synology Staging Images 30490007474.
  - Community Data Acceptance proved owner and non-owner behavior, privacy upper bounds, sanitized unavailable states, two concurrent real-MariaDB main-character writers and the complete zero-retry Chromium desktop/tablet/mobile lifecycle in English and Polish.
  - Product and route ledgers passed with 43 capabilities classified as 23 implemented, 3 partial, 14 missing and 3 not applicable.
  - PR #308 was squash-merged as 86847d0068e470274b6c3ee5523fe41cbb9663af and Issue #307 closed as completed; parent Issue #277 remains open for excluded mutation and achievement lifecycles.
derived:
  - Issue #307 is complete for its approved Platform-owned boundary and requires no Canary or production follow-up.
  - Rename, deletion/restore, world or channel transfer and authoritative achievement selection remain separate Issue #277 work requiring explicit contracts and authorization.
unknown:
  - Actual production deployment identity, database state, latency and recovery behavior remain unverified.
conflicts: []
first_failure:
  marker: none
  evidence: Every required exact-final-head workflow passed before PR #308 merged.
rejected_hypotheses:
  - Store owner comments or visibility preferences in Canary players data.
  - Reuse a generic Canary write principal for profile preferences.
  - Treat a stored preference row or browser-supplied identifier as ownership proof.
  - Allow character-level opt-in to override account-level privacy.
  - Close parent Issue #277 after only the Platform-owned profile slice.
changed_paths:
  - app/CharacterProfiles/**
  - app/Http/Controllers/CharacterProfiles/**
  - app/Http/Requests/CharacterProfiles/**
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - database/migrations/2026_07_29_165500_create_character_profile_preferences.php
  - routes/modules/character-profile-preferences.php
  - resources/views/identity/account/**
  - resources/views/game/character.blade.php
  - lang/{en,pl}/character_profiles.php
  - tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
  - scripts/acceptance/**character-profile**
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - .github/workflows/community-data-acceptance.yml
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md
validation:
  - command: Required exact-final-head workflow suite
    result: PASS
    evidence: All 11 workflow runs listed under proven completed successfully at 3797a094cfa522f5147d624786f49fee5027c77b before merge.
  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs
    result: PASS
    evidence: Portal Acceptance Contract 30490007458 validated all 43 capabilities.
  - command: node scripts/acceptance/coverage/validate-portal-coverage.mjs
    result: PASS
    evidence: Portal Acceptance Contract 30490007458 validated route ownership and stable evidence markers.
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: Agent Governance 30490007484 passed on the exact final head.
blockers: []
next_action: Continue parent Issue #277 only through a new bounded active task and separate pull request with explicit authorization for any Canary mutation; no further action remains for Issue #307.
```
'''


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f'{label}: expected one occurrence, found {count}')
    return text.replace(old, new, 1)


task = ACTIVE_TASK.read_text(encoding='utf-8')
task = replace_exact(task, '- [ ] PR merges and Issue #307 closes; parent Issue #277 remains open for the unapproved mutation lifecycles.', '- [x] PR merges and Issue #307 closes; parent Issue #277 remains open for the unapproved mutation lifecycles.', 'merge criterion')
task = replace_exact(task, '- [ ] Task is archived in a separate documentation PR.', '- [x] Task is archived in a separate documentation PR.', 'archive criterion')
task = task.replace('docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md', 'docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md')
task, count = re.subn(r'## Context checkpoint\n\n```yaml\n.*?\n```\n', FINAL_CHECKPOINT, task, count=1, flags=re.S)
if count != 1:
    raise RuntimeError(f'checkpoint: expected one block, found {count}')
ARCHIVE_TASK.write_text(task, encoding='utf-8')
ACTIVE_TASK.unlink()

active_work = ACTIVE_WORK.read_text(encoding='utf-8')
active_work = replace_exact(
    active_work,
    '- `OTERYN-20260729-character-profile-preferences` — deliver Issue #307 Platform-owned owner comments, per-character field visibility and one-main-character preference without Canary mutation; parent #277 remains open for rename/delete/restore/transfer.',
    '- None.',
    'active work entry',
)
closed_marker = '## Closed acceptance and release-preparation follow-ups\n\n'
closed_entry = '- PR #308 / `86847d0068e470274b6c3ee5523fe41cbb9663af` — delivered Platform-owned character comments, per-character privacy and optional main-character selection after all 11 exact-final-head workflows passed; Issue #307 closed while Canary remained read-only and parent #277 stayed open.\n'
active_work = replace_exact(active_work, closed_marker, closed_marker + closed_entry, 'closed work marker')
active_work = active_work.replace('The active task delivers Issue #307 as the Platform-owned privacy/comment/main-character slice of parent #277.', 'No task is currently active. PR #308 completed Issue #307; parent #277 remains open for separately contracted mutation and achievement work.')
ACTIVE_WORK.write_text(active_work, encoding='utf-8')

project = PROJECT_STATE.read_text(encoding='utf-8')
project = replace_exact(
    project,
    '`OTERYN-20260729-character-profile-preferences` in draft PR #308 delivers Issue #307, the Platform-owned profile-preference slice of parent #277, without Canary mutation.',
    'None. PR #308 completed Issue #307 and its task is archived; parent #277 remains open for separately contracted mutation and achievement work.',
    'project active task',
)
project = replace_exact(
    project,
    '1. Complete PR #308 / Issue #307 exact-head evidence and archive its task; keep parent #277 open for separately contracted mutation lifecycles.',
    '1. Continue parent #277 only through a new bounded task after selecting an explicitly authorized mutation or authoritative achievement scope.',
    'recommended sequence',
)
project = replace_exact(project, '## Character profile preferences candidate', '## Character profile preferences delivery', 'delivery heading')
project = replace_exact(
    project,
    'Draft PR #308 implements the Platform-owned Issue #307 slice: bounded escaped owner comments, per-character effective visibility, filtered related-character association, optional single main-character selection, audit events, real-MariaDB race evidence and EN/PL desktop/tablet/mobile browser acceptance. Canary remains read-only; rename, deletion, restore, transfer and selected achievements remain outside this contract, and no production claim is made.',
    'PR #308 completed the Platform-owned Issue #307 slice as merge `86847d0068e470274b6c3ee5523fe41cbb9663af`. Exact final head `3797a094cfa522f5147d624786f49fee5027c77b` passed all 11 required workflows, including real-MariaDB main-character concurrency and zero-retry EN/PL desktop/tablet/mobile browser acceptance. Canary remained read-only; rename, deletion, restore, transfer and selected achievements remain outside this contract under parent #277, and no production claim was made.',
    'delivery paragraph',
)
PROJECT_STATE.write_text(project, encoding='utf-8')

print('Archived character profile preference task and reconciled active/project indexes.')
