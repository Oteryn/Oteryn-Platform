from pathlib import Path

ACTIVE_WORK = Path('docs/agents/ACTIVE_WORK.md')
PROJECT_STATE = Path('docs/agents/PROJECT_STATE.md')


def replace_exact(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f'{label}: expected one occurrence, found {count}')
    return text.replace(old, new, 1)

active = ACTIVE_WORK.read_text(encoding='utf-8')
active = replace_exact(
    active,
    '- None.',
    '- `OTERYN-20260729-character-rename-contract` — deliver Issue #324 read-only discovery and the operation-specific Canary-safe rename contract under parent #277; no Canary mutation or runtime activation is authorized.',
    'active task entry',
)
marker = '## Closed acceptance and release-preparation follow-ups\n\n'
entry = '- PR #318 / `f90bb8075b300569b7d493c84f0080e6b3295c35` — archived the completed Issue #307 character-profile preference task, cleared ACTIVE_WORK and reconciled PROJECT_STATE after all seven archive-head workflows passed.\n'
active = replace_exact(active, marker, marker + entry, 'closed archive marker')
ACTIVE_WORK.write_text(active, encoding='utf-8')

project = PROJECT_STATE.read_text(encoding='utf-8')
project = replace_exact(
    project,
    'None. PR #308 completed Issue #307 and its task is archived; parent #277 remains open for separately contracted mutation and achievement work.',
    '`OTERYN-20260729-character-rename-contract` in Issue #324 is the active read-only discovery task for the operation-specific character-rename contract under parent #277; no Canary mutation is authorized.',
    'project active task',
)
project = replace_exact(
    project,
    '1. Continue parent #277 only through a new bounded task after selecting an explicitly authorized mutation or authoritative achievement scope.',
    '1. Complete Issue #324 read-only rename-contract discovery and decide whether a safe authorized implementation path exists before any Canary write task starts.',
    'recommended sequence',
)
PROJECT_STATE.write_text(project, encoding='utf-8')

print('Initialized Issue #324 active task indexes.')
