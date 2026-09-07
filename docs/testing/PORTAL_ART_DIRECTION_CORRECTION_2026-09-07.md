# Oteryn Portal — Art Direction Correction

Date: 2026-09-07  
Issue: #1297  
PR: #1298  
Branch: `feat/20260906-premium-portal-redesign`

## Owner decision

The existing PR implementation is retained as a functional/player-surface baseline, but it is **not accepted as the final visual redesign**.

The owner explicitly prefers the earlier cinematic portal concept shown in the owner-provided reference screenshot.

The reference screenshot itself should be attached to the Astra execution prompt. Ignore the surrounding chat UI, archive chips and editor overlays; focus on the Oteryn portal composition.

## What was wrong with the current implementation

The current implementation moved the product away from the legacy presentation and added a coherent design system, but major surfaces still read too much like a technical Laravel application/dashboard.

The main causes are:

- excessive reliance on bordered rectangular cards;
- too many equal-weight grids;
- insufficient image-led hierarchy;
- not enough full-bleed or layered world composition;
- account-style information architecture leaking into public/editorial surfaces;
- design validation optimized too heavily for route coverage, testability and responsive correctness instead of art-direction fidelity;
- the original cinematic concept was not treated as a binding qualitative visual acceptance target.

Passing tests, having no overflow and using shared tokens are necessary but are not sufficient for visual acceptance.

## Required target

The portal should feel like a finished premium MMORPG product.

Key qualities:

- cinematic fantasy world art integrated into the page architecture;
- strong Oteryn hero and world identity;
- dark blue/charcoal/black depth with restrained gold/ivory accents;
- premium display typography;
- atmospheric gradients, vignettes, texture and layered scenery;
- asymmetric editorial layouts;
- image-led news/discovery;
- world/server status integrated naturally with game identity;
- game-like ledgers for dense data;
- restrained ornament;
- purposeful mobile compositions;
- coherent family identity without forcing every route into the same card template.

Do not copy another MMORPG site. The reference is a quality/art-direction target, not a pixel-copy specification.

## Scope

The art-direction correction applies to the complete player-facing portal represented by the route/view ledger in:

`docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md`

This includes the global shell, homepage, editorial/events, servers/online/highscores, characters, guilds, Wiki/catalog, downloads, support, marketplace, identity/account/security/MFA, payments/player tools where player-facing, and error/empty/unavailable/maintenance states.

## Preserve

Do not change product semantics solely for presentation.

Preserve routes, controllers, models, auth/authz, CSRF/session/security, account ownership, payment behavior, support/enforcement semantics, truthful runtime states, localization contracts, forms/methods/field names and accessibility semantics.

## Visual completion rule

A rendered page is not complete merely because it works.

For every major page family, inspect a real Laravel screenshot and ask:

> Does this look like a polished commercial MMORPG portal, or a technical web application wearing a dark theme?

If it still reads as the latter, continue redesigning.

The exact execution prompt for the next autonomous visual implementation pass is:

`docs/agents/prompts/OTERYN-PORTAL-ART-DIRECTED-REDESIGN-ASTRA.md`

PR #1298 remains the owned redesign workstream until live repository authority says otherwise.
