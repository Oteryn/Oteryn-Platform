# Oteryn portal audit addendum — Wiki HTML pattern validation

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
This addendum extends the consolidated report and updates the normalized finding total to **one HIGH, five MEDIUM and one LOW**.

## OTERYN-AUDIT-P35-007 — Invalid HTML pattern weakens native validation on two Wiki administrator fields

```yaml
id: OTERYN-AUDIT-P35-007
title: Invalid HTML pattern weakens native validation on Wiki key fields
fact_state: PROVEN
severity: MEDIUM
confidence: HIGH
environment: REPO_MAIN plus historical CI_PROVEN browser evidence
surface: wiki.admin-editorial-lifecycle
capability: native client-side validation for category stable key and article content type
exact_sha: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
backend_status: protected by Laravel regex validation
frontend_status: two rendered inputs use a pattern that Chromium reports as an invalid regular expression
integration_status: historical desktop, tablet and mobile runs emitted console errors on both forms while valid submissions remained operable
state_coverage: invalid-value native constraint behavior on the frozen target was not directly rerun
impact: browser-native pattern validation can be disabled or unreliable and the administrator forms emit console errors; server-side integrity remains protected
recommendation: escape or reposition the hyphen in the HTML pattern and add a focused browser regression covering patternMismatch plus a zero-console-error assertion
suggested_followup_task: include with the smallest Wiki/UI remediation after exact-target validator classification
overlaps: [Issue #365 only as nearby evidence, not as a proven shared cause]
```

## Direct frozen-source evidence

The exact frozen target contains the same HTML pattern in two delivered administrator Wiki forms:

```html
pattern="[a-z0-9]+([._-][a-z0-9]+)*"
```

Locations:

- `resources/views/admin/wiki/categories/form.blade.php` — `key` / “Stable key”;
- `resources/views/admin/wiki/articles/form.blade.php` — `content_type` / “Content type”.

The backend independently enforces the intended grammar:

```php
regex:/^[a-z0-9]+(?:[._-][a-z0-9]+)*$/
```

Locations:

- `app/Wiki/Http/Admin/Requests/AdminWikiCategoryRequest.php`;
- `app/Wiki/Http/Admin/Requests/AdminWikiArticleRequest.php`.

This separates frontend constraint-validation quality from backend data integrity.

## Historical browser evidence

Both preserved Issue #365 artifacts recorded the Chromium console error:

```text
Pattern attribute value [a-z0-9]+([._-][a-z0-9]+)* is not a valid regular expression:
Invalid regular expression: /[a-z0-9]+([._-][a-z0-9]+)*/v:
Invalid character in character class
```

The error appeared twice in every responsive viewport execution:

- once on `/admin/wiki/categories/create`;
- once on `/admin/wiki/articles/create`.

The pattern was identical across historical heads `35f39b...`, `fb1bba...` and the frozen target source. Exact-target browser execution remains part of the independent validator packet, so this addendum does not claim a fresh frozen-target runtime reproduction.

## Severity rationale

`MEDIUM` is used because:

- the defect affects delivered administrator UI and native invalid-input feedback;
- it emits deterministic browser console errors across all representative responsive viewports;
- the same literal remains in frozen source;
- Laravel request validation still enforces the intended syntax, so there is no proven authorization, security or persisted-data bypass.

The finding is independent from the publication-flash and thumbnail-500 symptoms. No shared cause is claimed.
