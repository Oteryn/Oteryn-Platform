---
name: frontend-design
description: Optional visual design for a new Oteryn Platform screen or intentional redesign, only after explicit user authorization to use this skill or select relevant optional skills for the current task. Not for routine fixes, backend work or performance diagnosis.
---

# Oteryn Platform frontend design

Check current-task authorization in root AGENTS.md before using the procedure. Without a grant, continue ordinary work without this skill.

After authorization, read `references/upstream.md` for the pinned Anthropic design procedure, within the following Platform constraints:

- Inspect the affected route, Blade view and existing JS/CSS before choosing an implementation. Follow the Laravel/PHP versions, assets and design patterns already present; do not assume React or add a framework.
- Preserve server-side authorization, output escaping, CSRF, form validation, password-manager support and existing account/security contracts.
- Prefer established components for a bounded fix. Only redesign the requested surface; a visual task is not authority to reopen accepted architecture or product decisions.
- Use real content and check long Polish labels, keyboard focus, small and large viewports, contrast and reduced motion where applicable.
- Use existing screenshot/preview tools when available. A mockup is not functional or production E2E evidence.
- Do not add packages, fonts, asset providers or paid services solely because an example uses them. Preserve provenance and task scope.

This wrapper follows the repository license. The unmodified upstream reference retains its own Apache-2.0 license in `references/LICENSE.txt`.
