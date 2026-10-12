---
task_id: OTERYN-20261012-platform-pg-p6a
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
search_first:
  - app/GameAuth/NativeEvidence/NativeEvidenceHighWaterWitness.php
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - deploy/synology/scripts/lib.sh
optional_reads:
  - deploy/synology/scripts/generate-native-admission-key.sh
  - deploy/synology/runner/compose.yml
---

# OTERYN-20261012 Platform PostgreSQL P6a: trust safeguards design

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The lead referred to #1622, which does not exist in this repository. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Rewrite the P6a trust-safeguards design note that `PLATFORM_POSTGRES_MIGRATION_PLAN.md` requires before P6 step 4. It must fix:

- the provenance-only init;
- the durable attempt evidence outside the database and the witness directory, with its storage and binding;
- a proof that tells a failed publication apart from a restore;
- the negative test "a restore never synthesizes a profile";
- the exact implementation files and tests;
- the questions for the producer-contract owner and the owner.

It designs no missing-profile repair. This task is documentation only.

## Acceptance criteria

- [x] Provenance-only init: it creates and binds the store id, advances no floor, runs at the start of step 4, and its allowed and refused states are listed.
- [x] Attempt evidence: location, location checks, record format and binding (run id, store id, scope, key id). It is outside the database, the witness and every Platform volume or backup scope.
- [x] Classification and a proof of soundness for `failed-after-witness`, with its assumptions and limits.
- [x] The negative test "a restore never synthesizes a profile", fully specified.
- [x] Exact files and tests; questions for the producer-contract owner and the owner; no repair.
- [ ] Exact-head CI passes, the complete Codex review is addressed and no thread is unresolved.

## Ownership

```yaml
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p6a.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p6a.md
  - docs/agents/tasks/active/OTERYN-20261012-pg-p6-design-notes.md
modules:
  - operations-docs
dependencies:
  - PR 1567 (merged first draft of the note)
  - PR 1578 (merged P3 bootstrap; the init follows its end state)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T23:59:00Z
head: UNKNOWN
branch: claude/platform-pg-p6a-20261012
pr: 1589
status: waiting
context_routes:
  - operations
  - database
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p6a.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p6a.md
  - docs/agents/tasks/active/OTERYN-20261012-pg-p6-design-notes.md
proven:
  - Only publishTrustedKey moves the trust-state floor from null. The other registry writers throw in lockedProfile, the reconciler refuses a missing profile and a null floor, and NativeEvidenceSource only peeks trust state.
  - withNamespace holds the exclusive floor lock across DB::transaction, so a shared lock proves that no trust writer is in flight.
  - peek is not read-only, because it reaches reconcileStoreProvenance, which can create or re-bind provenance.
  - The runtime re-binds a marker-only state (producer contract, store provenance).
  - On local PostgreSQL 16, a non-superuser role can read system_identifier, timeline_id, postmaster start time, database oid, relfilenode, WAL LSN, snapshot xmax, pg_prepared_xacts and pg_stat_activity.
derived:
  - Equal database identity between the intent and the observation, with zero same-role open transactions, makes floor 1 with no profile a failed publication, unless privileged manual SQL deleted the rows.
unknown:
  - Whether the Synology share for the evidence can be kept out of every snapshot and restore task, and whether chattr +a works on it (owner questions O1 and O2).
conflicts:
  - The record OTERYN-20261012-pg-p6-design-notes.md (PR 1567, merged) listed this note as owned. This branch removes the note from its owned_paths. The lead authorized that edit. The rest of the file equals PR 1588's version (terminal_pr_policy archive_pending, and the P6b path already removed), so merging 1588 later only touches the lines this branch changed.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Keeping the evidence at /volume1/docker/oteryn/attempt-evidence (the draft's choice). It shares the docker share with the state directory and the database backups.
  - Keeping the evidence beside used.kids in the key volume. The P5 backup scope covers that volume.
  - Building the observation on peek. It can create provenance.
  - A guard-only identity check before publish-trust. A restore after the check could still be re-bound and witnessed (Codex P1), so the fence moved inside the publication transaction.
  - Directory chattr +a as the integrity mechanism. It does not protect file content (Codex P1), so a mandatory external anchor replaced it.
  - A create-and-remove probe in check-location. It fails on an append-only directory (Codex P1).
  - A fence made of caller-supplied fields (store, scope, kid, identity). Anyone who can run publish-key could fabricate one (Codex P1, round 2), so the producer now reads and verifies the named intent from a read-only evidence mount and takes every checked value, including the progress baseline (Codex P2), from that line.
  - Accepting the last unobserved intent without proof that its anchor was posted. A failed anchor post left a usable intent (Codex P1, round 3), so an intent is usable only once P6 appends its `anchored` line after the post succeeded.
  - Comparing Docker's reported bind `Source` strings and deriving the share from them. Moby does not resolve symlinks, so an alias could put the evidence inside the database restore scope (Codex P2, round 4); check-location now requires `Source` to equal its kernel-resolved `mountinfo` path.
  - Scoping restore detection to one store id. Restoring the database and platform_storage to before the init after a committed run let a new store S2 be created (Codex P1, round 5); the evidence directory is now bound to one store id, and a missing or different store is refused before the init as `store-changed`.
  - Checking anchors only in classify. A deleted run file emptied the directory before the continuity rule read it (Codex P1, round 6); `open` now verifies the fetched anchor set against the directory first and refuses with `evidence-lost`.
  - Writing the `init` record only after D1. A runner that died between D1 and that record left a store with no record, so a later restore looked like a first run (Codex P1, round 7); an anchored `init-intent` now precedes D1, and an unfinished one blocks `open` until `--close-init` closes it.
  - Closing a committed run with `done: false` after a failed self-check. Every later `open` was `step4-complete`, so the cutover could never finish (Codex P2, round 8); the run now stays open, and `step4-recheck` reruns only the read-only self-check and closes it with `done: true`.
changed_paths:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p6a.md
  - docs/agents/tasks/active/OTERYN-20261012-pg-p6-design-notes.md
validation:
  - command: python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run before push
blockers:
  - none
next_action: The lead and the owner review and accept the note, and answer P1 to P6 and O1 to O5. Then the P6a implementation packet can start. Archive this record in the final authoring commit.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository docs PR
source_branch_evidence: pending
```
