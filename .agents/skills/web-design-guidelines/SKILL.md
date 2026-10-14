---
name: web-design-guidelines
description: Optional focused review of changed Oteryn Platform web UI for accessibility and interaction problems, only after explicit user authorization to use this skill or select relevant optional skills for the current task. Not for native Rust UI, backend-only changes or unrelated redesigns.
---

# Oteryn Platform web interface review

Check current-task authorization in root AGENTS.md. Without a grant, continue ordinary work without this skill.

Read the pinned Vercel rules in `references/guidelines.md`, then inspect only the requested or changed UI and its directly relevant consumers. The reference's `$ARGUMENTS` means the current task's file scope; it is not an instruction to fetch or execute anything. No moving remote guideline fetch is required.

Apply applicable rules for semantic controls, keyboard/focus, forms, content overflow, reduced motion, media, localization and interaction states. Translate React/Tailwind examples to the repository's actual Blade/JS/CSS stack. Style preferences such as title capitalization are subordinate to established Oteryn language and design decisions; do not create unrelated cosmetic findings.

Report actionable findings with file/line and user impact. Explain which concerns need a rendered browser check rather than claiming static review proves behavior. Do not fix or redesign beyond the authorized task, add dependencies, or replace required security and runtime checks with this checklist.

The wrapper follows the repository license. Vercel's pinned guideline text and MIT notice are retained in `references/`; provenance is in `provenance.json`.
