# Portal visual polish — measured acceptance evidence

Date: 2026-09-08

Governing Issue: #1333. Pull request: #1334. Repository: `Oteryn/Oteryn-Platform`.

## Scope and evidence boundary

This report records engineering evidence for the visual-polish follow-up to the integrated Portal redesign. It does not claim production deployment, live-data verification, payment activation or a complete accessibility certification.

The measured repaired implementation was commit `937fb0c341aaff277102be59e1513ad399286122`. A later branch commit `1b9909342a303affe567fbfcb69193a2f2522a80` added one candidate-only image and independently passed the repository PR workflows, including CI, Portal Acceptance Contract, Acceptance E2E and Visual UX, Agent Governance, CodeQL, Edge Security Emulation, Platform DB Outage Validation, Game Auth Ticket Concurrency, Phase 7 Production-Like Validation, Content Scale Acceptance, Events Acceptance, Wiki Reconciliation Acceptance and Build Synology Staging Images.

## Verified repair outcomes

The mobile navigation regression was caused by the responsive disclosure handler closing a newly visible mobile menu during a queued media-query change. `public/js/portal-navigation.js` now closes only disclosures hidden by the resulting layout. The existing real keyboard/locale/search/no-JavaScript acceptance test was retained, and an additional deterministic breakpoint-order regression was added.

The three administrator routes `admin.audit.index`, `admin.homepage-templates.index` and `admin.roles.index` already existed and remained permission-protected. The strict route/view/navigation scanner needed explicit literal route references. The navigation partial now provides those references without route exemptions, scanner weakening or permission changes.

At repaired commit `937fb0c341aaff277102be59e1513ad399286122`:

- Acceptance E2E and Visual UX run `34205457055` passed. The separate PR reports executed 145 tests with zero failures or errors across smoke, Chromium/Firefox/WebKit portability, responsive, resilience and accessibility profiles.
- Portal Acceptance Contract run `34205457081` passed strict portal coverage closure and complete account lifecycle.
- CI run `34205457014` passed formatting, static analysis, runtime tests and `platform-gate`.
- CodeQL `34205457137`, Edge Security `34205457061`, DB Outage `34205457366`, Game Auth Concurrency `34205456840`, Phase 7 `34205456974`, Content Scale `34205457160`, Events `34205456852`, Wiki Reconciliation `34205457005` and staging-image build `34205457100` passed.

Acceptance artifact `10047843402` has recorded digest `sha256:e299f313a07bc747833709e143caea00342ce596b8f5298c190f31581ef9c1ce` and contains 206 real Laravel screenshot records across public, identity/account/security, commerce/support/error and administrator page families, including EN/PL and 320/390/820/1440/1920 px viewports. The manifest records no missing images, unlabelled controls or document-level horizontal overflow in those captures.

## Measured before and after

Baseline commit `4b71868c969766416c57246bf74186168375d125` used acceptance run `34194603808`, artifact `10043606384`, digest `sha256:b7096f699bc388d7514c73f07d2a40a404ff97f600565ced9e3be4a22a69e1c0`.

Compared with repaired commit `937fb0c`:

| Measurement | Before | Repaired |
| --- | ---: | ---: |
| Home decoded CSS bytes | 95,747 | 90,490 |
| News/Wiki/highscores/login decoded CSS bytes | 77,605 | 76,782 |
| Hero display width at 820 px viewport | 820 px | 626 px |
| Hero display width at 1440 px viewport | 1,094 px | 626 px |
| Hero display width at 1920 px viewport | 1,344 px | 626 px |
| Home document height at 390 px | 5,059 px | 4,877 px |
| Home document height at 820 px | 3,825 px | 3,525 px |
| Home document height at 1440 px | 2,763 px | 2,676 px |
| Home document height at 1920 px | 2,780 px | 2,688 px |

These are decoded stylesheet-body and controlled-layout measurements, not compressed transfer totals, Core Web Vitals or production speed claims. The baseline recorder did not force every lazy image to load, so total network request/time comparisons are intentionally not claimed.

## Visual implementation summary

- Repeated small scenery is no longer stretched as fictitious editorial media.
- The home discovery assets use explicit 240×100 dimensions and lazy loading.
- The primary 626 px artwork is capped at native width instead of being enlarged at tablet, desktop and wide sizes.
- News uses one decorative lead while actual posts remain truthful text content.
- Article/event reading surfaces remove unnecessary stretched decorative strips.
- Wiki featured/recent duplication is removed.
- Shared typography, spacing, density, controls, cards, tables, footer, account/security and administrator task-directory presentation are tightened without changing domain contracts.
- Administrator visibility is a read-only granted-permission projection; route middleware remains authoritative and role revocation remains regression-tested.

## Candidate artwork disposition

Commit `1b9909342a303affe567fbfcb69193a2f2522a80` added `public/images/oteryn-thais-concept.webp` as an intentionally unused candidate. GitHub reports blob `690d61bf1d8aeae66c89a9d0a047ea69e606f2a9`. A bounded binary-header check through the same connector path used for a known-good WebP found the candidate prefix does not have a WebP RIFF/WEBP header, while `public/images/oteryn-citadel.webp` does. Because the candidate was unused and not independently decodable as WebP, final Portal integration removes that broken public asset rather than wiring or shipping it. A valid source image may be evaluated in a separate future change.

## Remaining integration condition

The exact integration candidate must include current protected `main`, preserve the independent Synology task archive, pass fresh exact-head required checks, and enter the repository's protected integration path. Issue #1333 remains open until the PR is actually integrated and task/source-branch closeout is verified.
