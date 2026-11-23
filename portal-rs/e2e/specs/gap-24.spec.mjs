import {adminControlAgent, agentId, worldId} from '../support/admin-control-fixture.mjs';
import {test, expect} from '../fixtures.mjs';
import {csrfRejected} from '../helpers.mjs';
import {query, seededStaff} from '../staff-rbac-helpers.mjs';

test('staff can track a verified Console result and see agent readiness', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const localized = (pl, en) => lang === 'pl' ? pl : en;
  const actor = await seededStaff(page, lang, 'platform_admin');
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'", [actor]);

  const agent = adminControlAgent();
  await agent.connect();
  try {
    expect((await page.goto(`/admin/console/approvals?lang=${lang}`)).status()).toBe(200);
    await page.getByRole('navigation').getByRole('link', {name: localized('Dostęp', 'Access'), exact: true}).click();
    const authority = page.locator('main section').filter({
      has: page.getByRole('heading', {name: localized('Weryfikacja wyników agentów', 'Agent result verification')}),
    });
    await expect(authority).toContainText(`${agentId} · ${localized('Klucz aktywny', 'Key current')}`);
    const effects = page.locator('main section').filter({
      has: page.getByRole('heading', {name: localized('Operacje zmieniające stan gry', 'Game-changing operations')}),
    });
    await expect(effects).toContainText(localized('Niedostępne', 'Unavailable'));
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize().width);

    // The Access page may queue a policy read. Resolve it before requesting world details.
    for (const request of await agent.poll()) {
      expect(request.operation).toBe('admin.policy.get');
      await agent.complete(request);
    }
    await agent.heartbeat();
    const route = '/console/worlds';
    const params = new URLSearchParams({lang, agent: agentId, target: worldId});
    expect((await page.goto(`${route}?${params}`)).status()).toBe(200);
    const form = page.locator('main form:has(input[name=operation][value="world.get"])');
    const fields = await form.evaluate(node => Object.fromEntries(new FormData(node)));
    await csrfRejected(page, route, fields);
    const queued = await page.request.post(route, {
      form: fields, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
    });
    expect(queued.status()).toBe(303);
    const url = queued.headers().location;
    expect((await page.goto(url)).status()).toBe(200);
    await expect(page.locator('main')).toContainText(localized('Oczekiwanie na zapisany wynik', 'Awaiting a stored result'));
    const [request] = await agent.poll();
    expect(request.operation).toBe('world.get');
    expect(request.actor.admin_id).toBe(actor);
    expect(request.target.world_id).toBe(worldId);
    expect(agent.authentic(request)).toBe(true);
    await expect(agent.complete(request, {unexpected_field: true}))
      .rejects.toThrow('Admin control results returned HTTP 400');
    expect(await query('SELECT status FROM admin_control_requests WHERE request_id=$1', [request.request_id]))
      .toEqual([{status: 'delivered'}]);
    expect(await agent.complete(request)).toEqual({status: 'recorded'});
    expect((await page.reload()).status()).toBe(200);
    await expect(page.getByText(`${worldId} · Verified frontier`, {exact: true})).toBeVisible();
    expect(await query('SELECT status FROM admin_control_results WHERE request_id=$1', [request.request_id]))
      .toEqual([{status: 'succeeded'}]);

    await query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=(SELECT id FROM roles WHERE key='platform_admin')", [actor]);
    expect((await page.request.get(`/admin/console/access?lang=${lang}`)).status()).toBe(403);
  } finally {
    await agent.close();
  }
});
