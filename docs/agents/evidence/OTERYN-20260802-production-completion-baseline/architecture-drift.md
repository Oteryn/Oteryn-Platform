# Architecture, roadmap and module-catalogue drift

## Proven drift

The roadmap and module catalogue are no longer a complete representation of the merged product.

### Statuses behind merged delivery

- `EditorialMedia` remains `IMPLEMENTING` in the module catalogue, while later merged public/admin integrations and strict portal acceptance evidence establish a delivered repository boundary.
- `Wiki` remains `IMPLEMENTING`, while public EN/PL reads/search, editor administration, media integration and launch content were merged. Issue #365 is an intermittent feedback/validation defect, not evidence that the entire Wiki module is absent.
- `Wallet` and `Marketplace` remain `IMPLEMENTING`, while the complete Character Bazaar and append-oriented wallet/escrow lifecycle were merged and validated in isolated acceptance.
- The roadmap still presents Phase 8 as wholly deferred, but wallet/marketplace foundations already exist. What remains deferred is regulated provider funding, products/entitlements and real-money activation.
- Downloads, Events, Announcements, community data, richer profiles, support/moderation and Game Catalog work are represented mainly through later project-state/evidence records rather than the original module table.

### Missing dedicated module boundaries

The original module table does not give first-class ownership to several capabilities now required by programme #451:

- Products and entitlements: premium/VIP, packages, vouchers, expiry, revocation and fulfilment.
- Legal/privacy commerce: EU/Poland consumer presentation, payment-data privacy, retention, refund/complaint and invoice/tax ownership decisions.
- Operations/observability as an explicit module: release identity, shared sessions/cache, queues, mail delivery, metrics/alerts, backups/restores and on-call.
- Public edge as an explicit operational boundary: Cloudflare/Tunnel/TLS/redirect/HSTS/origin reachability.
- Quality/E2E as a durable delivery module: capability ledger, route reachability, browser states, exact-head and exact-deployment evidence.

### Misleading completeness interpretation

`AVAILABLE` means at least one validated capability exists; it does not mean every operation in the module exists. Therefore:

- `Characters: AVAILABLE` coexists with missing rename, delete/restore, world transfer and achievements.
- `Accounts: AVAILABLE` does not include arbitrary existing-account claim/import or an authoritative game-login bridge.
- `CMS: AVAILABLE` does not by itself prove every rich/media/content consumer and production delivery path.
- engineering Phase 7 completion remains `STAGING_PROVEN`; it does not establish private-production or public-go-live correctness.

## Required reconciliation

A later bounded documentation slice should:

1. update module statuses from `IMPLEMENTING` where merged evidence proves the current boundary;
2. add ProductsEntitlements, LegalCommerce, OperationsObservability, PublicEdge and QualityE2E module ownership;
3. split payments foundation from real-payment activation;
4. link each module to the machine-readable capability ledger and exact production evidence state;
5. ensure roadmap completion labels never imply `PRODUCTION_PROVEN`.

No architecture status should be upgraded merely from this audit narrative; the update must cite exact merged PRs and current ledger evidence.
