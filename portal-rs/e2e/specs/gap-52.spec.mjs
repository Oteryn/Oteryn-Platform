import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '../fixtures.mjs';
import { action, query, seededStaff } from '../staff-rbac-helpers.mjs';

test('an assigned investigator finds the incident response timeline', async ({ page, browser }, info) => {
  test.setTimeout(180000);
  const lang = info.project.metadata.locale;
  const index = '/admin/incidents';
  const otherContext = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
  });
  try {
    const investigator = await seededStaff(page, lang, 'platform_admin');
    const world = `incident-${randomUUID()}`.padEnd(128, 'x');
    const [{ id: alarm }] = await query(
      `INSERT INTO audit_log (action,target,before,after,request_id)
       VALUES ('ops.playerdata_binding_conflict','binding-reference','null',
               '{"record_id":"alarm-evidence-internal"}',uuidv7()) RETURNING id`,
    );

    expect((await page.goto('/admin')).status()).toBe(200);
    await page.locator(`main nav a[href="${index}"]`).click();
    await action(page, `form[method="post"][action="${index}"]`, {
      investigator, world_id: world, character_id: 'test-character', alarm_id: alarm,
    });
    await expect(page).toHaveURL(/\/admin\/incidents\/[0-9a-f-]{36}$/);
    const detail = new URL(page.url()).pathname;
    const caseId = detail.split('/').at(-1);

    // Persist observations as fixture data for the historical read path.
    const [{ request_id: preview }] = await query(
      `INSERT INTO admin_control_requests
       (request_id,admin_operation_id,operation,operation_version,kind,target,params,params_digest,
        dry_run,risk_class,actor,reason,approvals,issued_at,expires_at,nonce,signer_key_id,canonical_envelope,signature,status)
       VALUES (uuidv7(),uuidv7(),'world.retire',1,'command',
               jsonb_build_object('world_id',$1::text,'character_id','test-character'),'{}',
               decode(repeat('00',32),'hex'),true,'R4',$2,jsonb_build_object('ticket_ref',$3::text),'[]',
               now(),now()+interval '2 minutes',decode(repeat('00',16),'hex'),'test','x',
               decode(repeat('00',64),'hex'),'succeeded') RETURNING request_id`,
      [world, investigator, caseId],
    );
    const otherPage = await otherContext.newPage();
    const approver = await seededStaff(otherPage, lang, 'platform_admin');
    await query(
      `INSERT INTO admin_control_previews
       (id,requester,operation,permission,intent_digest,preview_digest,preview,reason)
       VALUES ($1,$2,'world.retire','game.ops.read',repeat('0',64),repeat('1',64),'{}','{}')`,
      [preview, investigator],
    );
    const [{ id: approval }] = await query(
      `INSERT INTO admin_control_approvals (id,preview_id,admin_id,step_up_at,approved_at)
       VALUES (uuidv7(),$1,$2,1,2) RETURNING id`,
      [preview, approver],
    );
    await query(
      `INSERT INTO audit_log (action,target,before,after,request_id)
       VALUES ('admin.control.transition',$1::uuid::text,'null','{"status":"succeeded"}',$1)`,
      [preview],
    );

    await page.goto(index);
    const search = page.locator('form[method=get]');
    await search.locator('[name=case_id]').fill(caseId);
    await search.getByRole('button').click();
    await expect(page).toHaveURL(new RegExp(`/${caseId}$`));
    const timeline = page.locator('section[aria-labelledby="incident-timeline"]');
    await expect(timeline).toContainText(lang === 'pl' ? 'Alarm o konflikcie powiązań' : 'Binding conflict alarm');
    await expect(timeline).toContainText(lang === 'pl' ? 'Zapisano niezależne zatwierdzenie' : 'Independent approval recorded');
    await expect(timeline).toContainText(lang === 'pl' ? 'Zakończono pomyślnie' : 'Succeeded');
    await expect(timeline).toContainText(approval);
    await expect(timeline).not.toContainText('alarm-evidence-internal');
    await expect(page.locator('section[aria-labelledby="incident-game"]')).toContainText(lang === 'pl' ? 'Niedostępne' : 'Unavailable');
    expect(await page.evaluate(() => document.documentElement.scrollWidth))
      .toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await otherPage.request.get(detail)).status()).toBe(404);
    expect((await page.request.get(`${detail}?from=2026-11-02&through=2026-11-01`)).status()).toBe(400);
    expect((await page.request.get(`${detail}?before=invalid`)).status()).toBe(400);

    await query(
      `INSERT INTO audit_log (action,target,before,after,request_id,created_at,actor)
       SELECT 'incident.assigned',$1,'null','null',uuidv7(),date_trunc('day',now(),'UTC'),uuidv7()
       FROM generate_series(1,101)`,
      [caseId],
    );
    expect((await page.reload()).status()).toBe(200);
    const first = await timeline.locator('ol > li').allTextContents();
    expect(first).toHaveLength(100);
    await timeline.locator('a[rel=next]').click();
    await expect(page).toHaveURL(/before=/);
    const remaining = timeline.locator('ol > li');
    await expect(remaining).toHaveCount(7);
    for (const event of await remaining.allTextContents()) {
      expect(first).not.toContain(event);
    }
    await expect(timeline.locator('a[rel=next]')).toHaveCount(0);
  } finally {
    await otherContext.close();
  }
});

test('i18n-parity for incident dossiers', () => {
  const labels = lang => new Map(
    [...readFileSync(new URL(`../../templates/admin/${lang}.ftl`, import.meta.url), 'utf8')
      .matchAll(/^(incident-[\w-]+)[ \t]*=[ \t]*(\S.*)$/gm)]
      .map(([, key, value]) => [key, value]),
  );
  const en = labels('en');
  const pl = labels('pl');
  expect([...pl.keys()].sort()).toEqual([...en.keys()].sort());
  const template = readFileSync(new URL('../../templates/admin/incidents.html', import.meta.url), 'utf8');
  for (const [, key] of template.matchAll(/self\.label\("(incident-[\w-]+)"\)/g)) {
    expect(en.get(key), key).toBeTruthy();
    expect(pl.get(key), key).toBeTruthy();
  }
});
