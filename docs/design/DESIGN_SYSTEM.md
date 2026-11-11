# Oteryn Platform Design System Foundation

## Status

Minimal durable design-system contract for the future production frontend.

The current `legacy/public/css/app.css` is a technical MVP and is not the design system described here. Implementation may evolve exact values through visual acceptance, but token names, semantic roles and component responsibilities should remain stable unless a later architecture decision supersedes them.

## Principles

1. **Semantic tokens before literals.** Components consume role-based tokens rather than hardcoded one-off colors.
2. **Shared foundations, distinct shells.** Public, Account and Admin reuse tokens/components while composing them differently.
3. **Accessible by default.** Focus, labels, contrast, error association and keyboard behavior are part of component contracts.
4. **Responsive by component.** A component owns its overflow/wrapping behavior; no component may rely on document-level horizontal scrolling.
5. **Progressive decoration.** Art is optional enhancement; core UI works with CSS/tokens alone.
6. **Server-rendered friendly.** Components must work cleanly with Blade and native HTML semantics without requiring a SPA runtime.

## Color tokens

Target palette:

```css
:root {
    --color-bg: #0B0D0F;
    --color-bg-subtle: #101316;
    --color-surface: #15191C;
    --color-surface-elevated: #1E2428;
    --color-surface-interactive: #252C31;

    --color-text-primary: #F3EBDD;
    --color-text-secondary: #B9B2A7;
    --color-text-inverse: #111315;

    --color-border: #3A352C;
    --color-border-strong: #5A4D39;

    --color-accent: #C7A86B;
    --color-accent-hover: #E1C889;
    --color-accent-active: #AD8D54;

    --color-success: #70B78A;
    --color-warning: #D5A34E;
    --color-danger: #D06C62;
    --color-info: #78A7C2;

    --color-focus: #F0C879;
    --color-overlay: rgb(0 0 0 / 0.62);
}
```

These values are the initial target, not permission to skip automated contrast validation. Text usage must remain constrained by semantic role.

### Color usage rules

- `--color-bg`: page canvas.
- `--color-surface`: primary content panel.
- `--color-surface-elevated`: cards, menus, dialogs, focused operation panels.
- `--color-text-primary`: body and heading text.
- `--color-text-secondary`: supporting/meta text only.
- `--color-border`: normal structural separators.
- `--color-accent`: brand/primary action signal.
- semantic success/warning/danger/info colors reinforce text/icons; they never carry meaning alone.

Do not introduce feature-specific colors when a semantic token already expresses the state.

## Typography tokens

Default implementation must work without a custom licensed font.

```css
:root {
    --font-interface: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    --font-display: Georgia, "Times New Roman", serif;

    --text-display: clamp(2rem, 4vw, 4rem);
    --text-page-title: clamp(1.75rem, 2.4vw, 2.5rem);
    --text-section-title: clamp(1.25rem, 1.6vw, 1.5rem);
    --text-body: 1rem;
    --text-small: 0.875rem;
    --text-meta: 0.8125rem;
    --text-table: 0.9375rem;

    --line-display: 1.05;
    --line-heading: 1.2;
    --line-body: 1.6;
    --line-compact: 1.35;
}
```

Rules:

- `display/brand`: hero/world identity only.
- `page title`: one primary H1 per screen.
- `section title`: H2/H3 according to semantic hierarchy.
- `body`: default readable copy.
- `small/meta`: timestamps, supporting metadata, never critical instructions.
- `table text`: compact but readable.

A future custom fantasy display font may replace only `--font-display` after licensing/performance/accessibility review.

## Spacing scale

Use a 4px-rooted scale:

```css
:root {
    --space-1: 0.25rem;
    --space-2: 0.5rem;
    --space-3: 0.75rem;
    --space-4: 1rem;
    --space-5: 1.25rem;
    --space-6: 1.5rem;
    --space-8: 2rem;
    --space-10: 2.5rem;
    --space-12: 3rem;
    --space-16: 4rem;
    --space-20: 5rem;
}
```

Prefer these values over arbitrary per-screen margins.

## Shape and elevation

```css
:root {
    --radius-sm: 0.375rem;
    --radius-md: 0.625rem;
    --radius-lg: 0.875rem;
    --radius-pill: 999px;

    --shadow-elevated: 0 18px 45px rgb(0 0 0 / 0.24);
    --shadow-focus: 0 0 0 3px rgb(240 200 121 / 0.35);
}
```

Use restrained elevation. Fantasy identity comes from composition/art/details, not from adding a different ornamental frame to every card.

## Layout tokens

```css
:root {
    --content-prose: 46rem;
    --content-standard: 72rem;
    --content-wide: 90rem;
    --content-admin: 96rem;
    --page-gutter: clamp(1rem, 3vw, 2rem);
}
```

- prose pages use `--content-prose` inside a wider shell when appropriate;
- standard public content uses `--content-standard`;
- table-heavy public views may use `--content-wide`;
- Admin may use `--content-admin`.

## Focus state

All keyboard-interactive components implement a visible `:focus-visible` state using `--color-focus` and/or `--shadow-focus`.

Do not remove browser outlines unless an equal-or-better replacement is present.

## Component contracts

### Button

Variants:

- `primary` — main positive action;
- `secondary` — alternative/navigation action;
- `ghost` — low-emphasis toolbar action;
- `danger` — destructive action.

States:

- default;
- hover;
- active;
- focus-visible;
- disabled;
- busy when an implementation actually has asynchronous behavior.

Requirements:

- minimum practical touch target around 44px where possible;
- destructive labels are explicit;
- no color-only distinction between primary and danger.

### Link

Variants:

- inline text link;
- navigation link;
- subtle/meta link;
- standalone action link.

Links remain visually distinguishable from surrounding text and expose focus state.

### Card

Use for a bounded content unit.

Properties:

- optional heading;
- optional metadata;
- optional actions;
- responsive internal padding;
- no fixed width that can break mobile.

### Panel

Use for structural sections such as Account status, World status or Admin work regions.

Panels may contain multiple cards/rows and should not be confused with decorative framing.

### Section Header

Contains:

- heading;
- optional short description;
- optional aligned primary/secondary action region.

On mobile, actions stack or wrap below the heading without forcing overflow.

### Input

Requirements:

- visible label;
- help text when needed;
- error text associated to the control;
- no global `min-width` larger than the viewport;
- full-width behavior inside narrow containers;
- clear focus/invalid/disabled states.

### Select

Uses the same height, typography, borders and focus treatment as Input.

Do not expose raw numeric/domain identifiers as user-facing labels when a validated display label exists.

### Checkbox

Requirements:

- native semantic input;
- visible text label;
- keyboard operable;
- adequate touch target;
- no custom styling that removes state clarity.

### Alert

Variants:

- info;
- success;
- warning;
- danger.

Structure:

- optional title;
- concise body;
- optional real recovery/action link.

Use `role="alert"` only for urgent dynamically relevant errors; use status semantics appropriately rather than applying ARIA indiscriminately.

### Badge

For compact categorical/status labels.

Examples:

- Draft;
- Published;
- Pending;
- Ready;
- Conflict.

Badge text is always present; color is reinforcement.

### Status Indicator

Structure:

- icon/shape;
- visible label;
- optional supporting text.

Examples:

- Server Online;
- Runtime Unknown;
- MFA Enabled.

### Table

Desktop contract:

- semantic `table`, `thead`, `tbody`, headers;
- sensible column alignment;
- actions grouped consistently;
- long content bounded.

Responsive contract is defined in `RESPONSIVE_STRATEGY.md` and must use local containment or row/card transformation. The table component must never widen the whole document.

### Pagination

Structure:

- Previous/Next controls;
- current-page indication;
- compact numbered links when useful;
- semantic navigation label.

Mobile may simplify the number range but must preserve orientation.

### Character Row/Card

Target data only when authoritative fields exist.

Possible regions:

- name;
- level/vocation/world only when actually available;
- status metadata;
- `View` action.

The component cannot invent absent fields.

### Server Status

Structure:

- status label;
- last-known/freshness context where authoritative;
- players online/current channel information where available;
- explicit unknown/unavailable state.

Never present missing dependency data as zero/offline unless that is the authoritative meaning.

### Empty State

Structure:

- short heading;
- explanation;
- one relevant next action when a real destination exists.

Examples:

- no published news;
- no online players;
- no characters;
- empty Admin list.

### Error State

Structure:

- safe title;
- plain-language explanation;
- request/retry guidance when appropriate;
- one or two real recovery actions;
- optional correlation/request ID only if product policy intentionally exposes a safe support reference.

Never expose stack traces, SQL, exception classes, secrets or internal paths.

### Navigation Item

States:

- default;
- hover;
- active/current;
- focus-visible;
- disabled only for genuine non-interactive explanatory items; do not render fake future links as disabled clutter.

Current location is conveyed with more than color.

### Account Navigation

Target sections:

- Overview;
- Characters;
- Security;
- Settings only when delivered.

On focused operations, show the nearest parent context and return path.

### Admin Navigation

Target groups:

- Dashboard;
- Content;
- Access;
- Operations.

Only authorized/current destinations render.

Desktop: persistent sidebar where space permits.
Tablet/mobile: accessible drawer/collapsible navigation.

## Form composition

Forms use:

```text
Page/Section heading
Optional explanation
Alert/validation summary
Field group
Field help/error
Actions: Primary | Secondary/Cancel
```

Do not place every security action into one giant Account form.

Destructive forms receive spatial separation and danger treatment.

## Secure-information component

Used for MFA manual secret/provisioning data and recovery codes.

Requirements:

- bounded wrapping or local scrolling for long values;
- monospace only for the secret/code value, not the full page;
- copy affordance only when implemented without weakening security semantics;
- clear one-time/save warning for recovery codes;
- print/download affordance only after explicit security review;
- no secret value in analytics/logging attributes.

## Flash and validation vocabulary

Use consistent categories:

- operation succeeded;
- action requires correction;
- action unavailable/dependency issue;
- authorization denied.

Avoid generic `status` styling that looks identical for success and failure.

## Icon strategy

Prefer:

- a small maintained first-party SVG icon set;
- simple consistent stroke/fill style;
- decorative icons hidden from assistive technology when labels already convey meaning.

Do not use emoji as the only production status iconography.

## Component implementation rule

Before introducing a new visual primitive, determine whether it is:

1. a reusable design-system component;
2. a shell/layout composition;
3. a page-specific content pattern.

Only category 1 belongs in the shared component vocabulary. Avoid turning every page fragment into an abstraction.
