# ADR 0026: Proprietary repository licensing policy

- Status: Accepted
- Date: 2026-08-06
- Decision owner: repository owner
- Decision record: Issue #587 / `ARCH-DEC-0002`
- Accepted option: A
- Canonical notice: `LICENSE.md`, revision 1

## Context

Oteryn Platform is publicly readable on GitHub, while `composer.json` already declares the package `proprietary`. Before this decision, the repository had no canonical license file, no durable distribution-rights policy and no complete provenance boundary for dependencies, assets, game data, fixtures or protocol-derived material.

Public visibility alone is not an intentional product decision about copying, modification, redistribution, hosted operation, commercial reuse or contribution rights. Publishing an open-source or source-available license before the repository owner selected a model and before provenance was reviewed could grant rights that the project did not intend or did not have authority to grant.

The decision backlog presented three alternatives:

- **A — proprietary or no-permission policy**;
- **B — owner-selected source-available terms after legal and provenance review**;
- **C — an exact OSI-approved license after compatibility and provenance review**.

## Decision

The repository owner explicitly accepted **Option A** on 2026-08-06.

Oteryn Platform is publicly readable but proprietary. The current repository policy is:

- all rights in original Oteryn Platform material are reserved by their respective rights holders;
- repository visibility, cloning, forking, Issue discussion or pull-request review does not grant copying, modification, redistribution, sublicensing, hosted-operation or commercial-use rights beyond applicable law and platform terms;
- `LICENSE.md` revision 1 is the canonical repository notice;
- `composer.json` remains declared `proprietary` and no SPDX open-source identifier is asserted;
- third-party dependencies, assets, game data, fixtures, protocol or compatibility material and files with separate notices are excluded from any Oteryn Platform ownership or relicensing claim;
- `THIRD_PARTY_NOTICES.md` records the current provenance boundary and requires unresolved material to fail closed before distribution;
- external contributions are not accepted by default and require prior written invitation plus documented contribution terms before acceptance;
- a later source-available, open-source, dual-license or component-specific license requires a new owner-approved ADR and evidence that the affected material may lawfully be licensed under those terms.

## Consequences

### Positive

- External readers receive an explicit answer instead of having to infer rights from repository visibility.
- The project does not unintentionally grant competitors permission to copy, host or commercially redistribute the Platform.
- Third-party rights are preserved rather than silently absorbed into a project-wide notice.
- Future selective open sourcing remains possible through a separately reviewed component boundary.
- Contribution intake cannot silently create uncertain inbound rights.

### Negative

- External reuse and community forks are not permitted without written permission.
- Unsolicited contributions cannot be accepted through an informal pull-request process.
- A provenance audit is still required before release distributions can claim complete third-party compliance.
- GitHub may not identify a standard license because the decision intentionally uses a proprietary notice rather than an SPDX open-source license.

## Rejected shortcuts

- Copy a license from another Oteryn, Canary, OpenTibia or unrelated repository without proving that its ownership and distribution assumptions apply here.
- Interpret public visibility or GitHub forking as an open-source grant.
- Name a legal entity or copyright holder not proven by authoritative repository evidence.
- Claim ownership of third-party dependencies, assets, game data or protocol-derived material.
- Accept external contributions first and determine contribution rights later.
- Treat `composer.json` metadata alone as a complete legal notice.

## Activation and implementation boundary

This ADR becomes authoritative when the decision package containing it merges to `main`.

It changes repository documentation and governance only. It does not perform a production deployment, publish a release, resolve all third-party provenance, grant rights in another repository or authorize the use of third-party intellectual property.

## Supersession

This decision remains in force until an explicit later ADR supersedes it. A later decision may license an independently bounded component under different terms only after proving ownership, provenance, compatibility and the exact files covered.

## References

- Issue #587
- PR #690
- `LICENSE.md`
- `THIRD_PARTY_NOTICES.md`
- `README.md`
- `CONTRIBUTING.md`
- `composer.json`
- `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json`
