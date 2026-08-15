# Tibia research ownership reconciliation — 2026-08-15

Coordination ID: `OTER-CLIENT-REFERENCE-HARVEST-20260815`.

## Decision

The stale research does not have one uniform owner.

| Source | Durable surface | Canonical disposition |
|---|---|---|
| Platform PR #988 | official-package identity hashing, dedicated-host preflight, fail-closed LUKS evidence setup, non-execution tests | **Platform infrastructure/reference harness** — harvest cleanly to current Platform lineage |
| Platform PR #988 | client/game conclusions and future implementation handoff | **Oteryn-v2 / Oteryn-Game lineage** — evidence only; no new work in `blakinio/otclient` |
| Platform PR #1006 | `tools/tibia-worldmap-reconstruction/**` | **Oteryn-v2 / Oteryn-Game lineage** — migrated by Oteryn-v2 PR #283 |
| Platform PR #1006 | Platform runner/container orchestration and temporary live-client workflows | **execution history only** — do not merge |
| Platform PR #1006 | screenshots/base64, credentials paths, VNC, gdb/ptrace/live attach, private-message and blind movement experiments | **do not migrate** — keep only historical PR provenance |

## Authority

### FACT — current client/game owner

Platform ADR 0041 assigns the native Client, authoritative Game Server, `protocol-oteryn`, native client/server/protocol E2E mechanics, canonical World/Content and bounded OTBM migration semantics to the Oteryn-Game lineage. It identifies current `blakinio/Oteryn-v2` as the target source lineage.

Oteryn-v2 ADR-0002 independently records the Rust client cutover to `blakinio/Oteryn-v2/apps/client`. `blakinio/otclient` is historical migration/reference evidence and is not a target for new Oteryn v2 work.

### FACT — prior drift was already identified

Platform Issues #864 and #886 previously proved that post-cutover handoffs targeting historical `blakinio/otclient` were stale. Issue #886 explicitly distinguished the still-useful synthetic/no-network harness from the incorrect implementation handoff target.

## PR #988 audit

Source PR: `blakinio/Oteryn-Platform#988`.
Source head: `f9ff34b37cf81c400a48f7ab9329393416ac304d`.

The branch has eight changed files. The blocked lifecycle requires a dedicated normal Linux graphical host, encrypted private evidence storage, normal official browser acquisition and offline no-network execution. Those acceptance steps are not complete and must not be represented as completed.

The following implementation is self-contained, non-executing/reference infrastructure and is harvested onto a clean current-main branch:

- `tools/tibia-linux-reference/official_identity_probe.py`;
- `tools/tibia-linux-reference/official_host_preflight.py`;
- `tools/tibia-linux-reference/official_host_prepare.sh`;
- `tools/tibia-linux-reference/official_evidence_luks_setup.sh`;
- `tools/tibia-linux-reference/tests/test_official_offline.py`;
- `.github/workflows/tibia-linux-official-identity.yml` with GitHub Actions pinned to immutable SHAs.

The task/report state from #988 is not copied as an active completed feature because actual dedicated-host execution remains blocked. The original Issue #987 remains historical authorization/provenance for that research phase until source closeout is recorded.

## PR #1006 audit

Source PR: `blakinio/Oteryn-Platform#1006`.
Source head: `97f8df9e64e1e4f0520440073e497f24dad929ef`.

The PR contains 302 commits and 76 changed files. The majority are experimental `.github/workflows/tibia-client-*` runner/live-client probes. The PR itself states its one-shot research workflow is temporary and must be removed before terminal merge/closeout.

### Durable artifact

The six-file `tools/tibia-worldmap-reconstruction/**` package is proprietary-data-free and independent of the Platform runner. It implements:

- strict normalized worldmap document validation;
- explicit observed/unknown semantics;
- monotonic merge with same-sequence conflict rejection;
- coordinate/static-stack comparison;
- fail-closed mapping states;
- OTBM export planning that refuses readiness for unobserved, unmapped or ground-unproven tiles.

That package is migrated unchanged in executable semantics to Oteryn-v2 PR #283. Its README is updated only to describe current provenance/ownership.

### Deliberately excluded

No branch-only live-client workflow is promoted to `main`. In particular, the migration excludes login/credential injection, VNC, private-message actions, gdb/ptrace/live-attach experiments, blind batched movement, raw screenshots/base64 and proprietary client material.

## Remaining research questions

The following are still UNKNOWN and are not acceptance criteria of the harvest:

- exact player XYZ structure/API independent of viewport-center inference;
- stable semantic mapping of raw captured Worldmap fields;
- reliable tile passability/collision classification;
- exact outbound writer/framing chain and higher-level action ABI;
- exact OTBM-relevant coverage recoverable from official-client state;
- exact current official Linux archive/client identity and BattlEye offline behavior.

Any continuation of live official-client research requires a new bounded owner-authorized task in the correct repository and must not be inferred from this migration.

## Closeout gate

PRs #988 and #1006 must remain unmerged. After the clean Platform harvest and Oteryn-v2 PR #283 are validated and merged, their PR bodies must receive `Branch-Disposition: delete` with a non-empty reason, then the PRs should be closed without merge and their exact source refs removed through the trusted branch-lifecycle control.
