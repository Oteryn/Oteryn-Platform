import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected } from '../helpers.mjs';
import { query, role, seededStaff } from '../staff-rbac-helpers.mjs';

const path = '/admin/audit-evidence';

test('staff can verify and download a closed day of Platform audit evidence', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const title = lang === 'pl' ? 'Dowody audytowe' : 'Audit evidence';
  test.setTimeout(180000);
  const actor = await seededStaff(page, lang, 'platform_admin');
  const target = await role(page);
  const [{ request_id: requestId }] = await query(
    "SELECT request_id FROM audit_log WHERE actor=$1 AND target=$2 AND action='rbac.role.created'",
    [actor, target],
  );
  await page.goto('/admin');
  await expect(page.getByRole('link', { name: title })).toBeVisible();
  await page.getByRole('link', { name: title }).click();
  await expect(page.getByRole('heading', { name: title })).toBeVisible();
  expect(await page.locator('body').evaluate(body => body.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  const offset = ['pl-1440', 'pl-390', 'en-1440', 'en-390'].indexOf(info.project.name) + 1;
  const [{ day, today, expired }] = await query(
    `SELECT (utc - candidate.days)::text AS day, utc::text AS today, (utc - 91)::text AS expired
     FROM (SELECT (now() AT TIME ZONE 'UTC')::date AS utc) AS calendar
     CROSS JOIN generate_series($1::int, 90, 4) AS candidate(days)
     WHERE NOT EXISTS (SELECT 1 FROM audit_evidence_seals WHERE day = utc - candidate.days)
     ORDER BY candidate.days LIMIT 1`,
    [offset],
  );
  // The production form closes days by UTC. Re-create the event through the
  // database capture boundary with a test clock equivalent to the previous day.
  await query(
    `INSERT INTO audit_log (actor, action, target, before, after, request_id, created_at)
     SELECT actor, action, target, before, after, request_id, $2::date::timestamp AT TIME ZONE 'UTC'
     FROM audit_log WHERE request_id = $1 AND action = 'rbac.role.created'`,
    [requestId, day],
  );
  await csrfRejected(page, path, { day });
  const form = page.locator(`main form[action="${path}"]`);
  const csrf = await form.locator('input[name=_csrf]').inputValue();
  const origin = new URL(page.url()).origin;
  for (const [requested, status] of [['2026-02-30', 400], [today, 404], [expired, 404]]) {
    const refusal = await page.request.post(path, { form: { day: requested, _csrf: csrf }, headers: { origin } });
    expect(refusal.status()).toBe(status);
    expect(refusal.headers()['cache-control']).toBe('no-store');
  }
  await form.locator('input[type=date]').fill(day);
  const [download] = await Promise.all([page.waitForEvent('download'), form.getByRole('button').click()]);
  const evidence = JSON.parse(readFileSync(await download.path(), 'utf8'));
  expect(evidence.day).toBe(day);
  expect(evidence.entries.map(entry => entry.record)).toContainEqual(expect.objectContaining({
    actor, target, action: 'rbac.role.created', request_id: requestId,
  }));
  let head = createHash('sha256').update(`oteryn-audit-evidence-v1\n${day}`).digest('hex');
  for (const [index, entry] of evidence.entries.entries()) {
    expect(entry.sequence).toBe(index + 1);
    expect(entry.previous_hash).toBe(head);
    const canonical = JSON.stringify(Object.fromEntries(Object.entries(entry.record).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)));
    head = createHash('sha256').update(head + '\n' + canonical).digest('hex');
    expect(entry.hash).toBe(head);
  }
  expect(evidence.head).toBe(head);
  const [{ anchored, entry_count: count }] = await query(
    "SELECT encode(head, 'hex') AS anchored, entry_count FROM audit_evidence_seals WHERE day=$1",
    [day],
  );
  expect(anchored).toBe(head);
  expect(count).toBe(evidence.entries.length);
  const [exported] = await query(
    "SELECT actor, after->>'head' AS head FROM audit_log WHERE action='audit.evidence.exported' AND target=$1",
    [day],
  );
  expect(exported).toEqual({ actor, head });
  const restricted = await browser.newContext({ ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport, baseURL: origin });
  try {
    const staff = await restricted.newPage();
    await seededStaff(staff, lang, 'support');
    await staff.goto('/admin');
    await expect(staff.getByRole('link', { name: title })).toHaveCount(0);
    expect((await staff.goto(path)).status()).toBe(403);
  } finally {
    await restricted.close();
  }
});

test('i18n-parity for audit evidence', () => {
  const keys = lang => [...readFileSync(new URL(`../../templates/admin/${lang}.ftl`, import.meta.url), 'utf8')
    .matchAll(/^(audit-evidence-[\w-]+)\s*=/gm)].map(match => match[1]).sort();
  expect(keys('en').length).toBeGreaterThan(0);
  expect(keys('pl')).toEqual(keys('en'));
});
