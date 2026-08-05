# Oteryn Platform current system/module reconciliation

## Identity

```yaml
task: OTERYN-20260805-system-module-reconciliation
issue: 593
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
repository: blakinio/Oteryn-Platform
exact_base: bc9f64ac78b7f6483a8b0679c422cf772ca20ad6
classification: documentation_drift
runtime_change: false
```

## Primary finding

The canonical module table and system-context diagram lag merged repository delivery. PR #453 recorded this drift and required a later exact-evidence reconciliation rather than a narrative-only status upgrade.

## Evidence classification

### PROVEN — available bounded modules

- EditorialMedia: PR #176 merged the secure private image library; later Wiki integration and portal acceptance prove an active consumer boundary.
- Wiki: PR #194 merged public EN/PL reads/search, PR #196 merged trusted administration and PR #199 merged EditorialMedia integration.
- Wallet and Marketplace: PR #270 merged the Oteryn Coins ledger, reservations, Character Bazaar, transfer saga, public/account/admin UI and isolated acceptance.
- Marketplace staging: PR #368 merged a staging-only enablement/control package. It is staging evidence, not production proof.
- GameCatalog: the detailed current catalogue already records a supported inactive import/activation boundary for schemas 1.0.0 through 1.2.0; the top module table omits the owner.

### PROVEN — incomplete or blocked dimensions remain

- Issue #365 preserves focused Wiki flash/thumbnail investigation without invalidating the delivered Wiki boundary.
- Issue #488 owns Wiki expected-content, failure/recovery and portability completeness gaps.
- Issue #489 owns Game Catalog, marketplace, product and provider-payment completeness gaps.
- Issue #490 owns PlatformAPI, operations and public-edge applicability/evidence gaps.
- Production-completion baseline evidence remains frozen historical proof and is not edited by this task.

### DERIVED — status model

A single status cannot safely represent implementation, completeness and environment proof. The focused canonical catalogue should therefore use:

1. module status for repository implementation availability;
2. explicit current-boundary prose for what is delivered;
3. linked exact evidence and open-gap Issues for completeness;
4. separate staging/production evidence labels for environment claims.

`AVAILABLE` means at least one documented capability is merged and validated on `main`. It never means product-complete, staging-proven or production-proven.

## Reconciliation decisions

1. Upgrade EditorialMedia, Wiki, Wallet and Marketplace from `IMPLEMENTING` to `AVAILABLE`.
2. Add GameCatalog as `AVAILABLE` because a validated bounded import/projection capability exists on `main`; keep schema 1.3 PR #338 explicitly outside current availability because it remains open and inactive.
3. Add ProductsEntitlements and LegalCommerce as `PLANNED` ownership boundaries.
4. Add OperationsObservability, PublicEdge and QualityE2E as `AVAILABLE` repository boundaries, while explicitly retaining environment proof gaps and Issue #490 ownership.
5. Keep PlatformAPI `PLANNED` and Payments `PLANNED-LATER`.
6. Keep Wallet/Marketplace independent from provider Payments and ProductsEntitlements.
7. Update the system diagram and dependency rules without implying a new deployable service or production state.

## Rejected alternatives

### Keep stale statuses until all open Issues close

Rejected. That would conflate existence with completeness and contradict the catalogue's own `AVAILABLE` definition.

### Mark delivered modules complete

Rejected. Open audit findings and deferred capabilities remain material; neither `AVAILABLE` nor the system diagram is a completion claim.

### Collapse Wallet, Marketplace, Products and Payments into commerce

Rejected. It would obscure different security, data, regulatory and activation boundaries.

### Create a new ADR

Rejected. No durable architectural choice changes. This task applies the already accepted authority order and evidence-dimension separation from ADR 0022 and PR #453.

## Scope and safety

Only the two focused canonical documents plus task/programme/report records may change. Runtime, migration, dependency, workflow, deployment, production, frozen evidence and external repositories remain untouched.

## Validation plan

- verify exact changed paths and links;
- search final canonical text for stale `IMPLEMENTING` and `Future Payments` claims;
- confirm open audit Issues remain referenced as gaps rather than closed;
- run exact-head Agent Governance and repository documentation/CI checks;
- perform a fresh contradiction and PR-hygiene audit;
- runtime E2E is `NOT_APPLICABLE` because no executable behavior changes.
