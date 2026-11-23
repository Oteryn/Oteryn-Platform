import {uuid7} from '../staff-commerce-helpers.mjs';
import {test,expect} from '../fixtures.mjs';
import {csrfRejected,register,submit,waitForLoginBudget} from '../helpers.mjs';
import {audit,query} from '../staff-rbac-helpers.mjs';
import {bootstrapAdmin,completeEnrollment,grantPlatformAuditRole,grantSeniorRole,requestForm} from '../journey-gamemaster-helpers.mjs';

const operations = [
  {path:'dashboard', operation:'ops.overview.get', label:['Pulpit','Dashboard']},
  {path:'worlds', operation:'world.list', label:['Światy','Worlds']},
  {path:'worlds', operation:'world.get', label:['Szczegóły świata','World details'], target:'cold-world'},
  {path:'nodes', operation:'node.list', label:['Węzły i hosty','Nodes and hosts']},
  {path:'nodes', operation:'node.get', label:['Szczegóły węzła','Node details'], target:'cold-node'},
  {path:'database', operation:'db.status', label:['Baza danych','Database']},
  {path:'database', operation:'db.migrations.list', label:['Migracje','Migrations']},
];

const analytics = ['', '/creatures', '/activity', '/levels', '/health', '/economy'];

test('senior GM role assignment, MFA, and cold console journey', async ({browser,page}, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const admin = await bootstrapAdmin(browser, info);
  const signedOut = await browser.newContext();
  const language = lang === 'pl' ? 0 : 1;
  const unavailable = ['Niedostępne','Unavailable'][language];
  const get = (path, options = {}) => page.request.get(path, {
    maxRedirects:0,
    headers:{'accept-language':lang},
    ...options,
  });

  let gmId;
  try {
    const account = await test.step('Register and verify the future GM in a separate browser context', async () => {
      const account = await register(page, lang);
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
      await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl'
        ? 'Twój adres e-mail został zweryfikowany.' : 'Your e-mail has been verified.');
      const response = await get('/admin');
      expect(response.status()).toBe(303);
      expect(response.headers().location).toBe('/login');
      return account;
    });

    const accountId = await test.step('Platform admin grants only game_master_senior through the Roles form', async () => {
      const [{id}] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
      gmId = id;
      expect((await admin.page.goto('/admin')).status()).toBe(200);
      await admin.page.getByRole('link', {name:lang === 'pl' ? 'Role' : 'Roles', exact:true}).click();
      await grantSeniorRole(admin.page, id);
      return id;
    });

    const role = await test.step('Platform admin creates and grants the Platform audit role through the Roles form', async () => {
      return grantPlatformAuditRole(admin.page, accountId, lang);
    });

    await test.step('GM sees the localized mandatory MFA refusal, enrolls, and completes login', async () => {
      await page.locator('#portal-header a.om-brand').click();
      const signIn = page.locator(`a.om-login[href="/${lang}/login"]`);
      if (info.project.use.viewport.width === 390) {
        const menu = page.locator('#portal-header.is-enhanced #portal-menu-toggle');
        await expect(menu).toHaveAttribute('aria-expanded', 'false');
        await menu.click();
        await expect(menu).toHaveAttribute('aria-expanded', 'true');
      }
      await signIn.click();
      await waitForLoginBudget();
      await submit(page, 'main form', {email:account.email, password:account.password});
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
      const response = await get('/admin');
      expect(response.status()).toBe(403);
      expect(await response.text()).toContain(lang === 'pl'
        ? 'Wymagane uwierzytelnianie wieloskładnikowe'
        : 'Multi-factor authentication required');
      await completeEnrollment(page, lang, account);
    });

    await test.step('GM cannot request Platform administration pages', async () => {
      for (const path of [
        '/admin/roles', '/admin/console/access', '/admin/console/approvals',
        '/admin/world', '/admin/news', '/admin/pages', '/admin/events',
        '/admin/media', '/admin/wiki', '/admin/moderation/enforcement', `/admin/${lang}/bazaar`,
      ]) {
        expect.soft((await get(path)).status(), path).toBe(403);
      }
    });

    const tokens = await test.step('Enter the staff panel and obtain session-bound CSRF tokens', async () => {
      const origin = new URL(page.url()).origin;
      const anonymous = await signedOut.newPage();
      expect((await anonymous.goto(`${origin}/${lang}/login`)).status()).toBe(200);
      const tokens = {
        gm:await page.locator('form[action$="/logout"] input[name="_csrf"]').first().inputValue(),
        admin:await admin.page.locator('main input[name="_csrf"]').first().inputValue(),
        anonymous:await anonymous.locator('main input[name="_csrf"]').inputValue(),
        anonymousPage:anonymous,
      };
      // The staff panel and Game console are separate direct entry points.
      expect((await page.goto('/admin')).status()).toBe(200);
      await expect(page.locator('main h1')).toHaveText(['Panel zespołu','Staff panel'][language]);
      await expect(page.locator('main a[href="/admin/roles"]')).toHaveCount(0);
      return tokens;
    });

    await test.step('The commerce shell and its actions refuse the GM role', async () => {
      const shell = await get('/admin/commerce');
      expect(shell.status()).toBe(403);
      expect(await shell.text()).not.toMatch(/<form\b[^<>]*\baction="\/admin\/commerce\//i);
      const form = {request_id:uuid7(), target:accountId, reason:'Journey permission check'};
      for (const [action, fields] of [
        ['grant', {days:'1'}], ['revoke', {}], ['adjust', {amount:'1'}],
      ]) {
        const response = await requestForm(page, `/admin/commerce/${action}`, {...form, ...fields}, tokens.gm);
        expect(response.status(), action).toBe(403);
      }
    });

    await test.step('Select and refresh the four Game console pages, including world and node details', async () => {
      expect((await page.goto(`/console/dashboard?lang=${lang}`)).status()).toBe(200);
      const nav = page.locator('header nav.op-tabs');
      await expect(nav.locator('a')).toHaveText(lang === 'pl'
        ? ['Przegląd gry', 'Światy', 'Węzły', 'Baza danych']
        : ['Game overview', 'Worlds', 'Nodes', 'Database']);
      for (const op of operations) {
        await nav.locator(`a[href="/console/${op.path}?lang=${lang}"]`).click();
        const selection = page.locator('main form[method="get"]');
        const agent = selection.locator('[name="agent"]');
        const target = selection.locator('[name="target"]');
        await agent.fill('bad/agent');
        expect(await agent.evaluate(input => input.validity.patternMismatch)).toBe(true);
        await agent.fill('cold-agent');
        await target.fill('bad/target');
        expect(await target.evaluate(input => input.validity.patternMismatch)).toBe(true);
        await target.fill(op.target || '');
        await selection.getByRole('button', {name:['Pokaż zapisane dane','View stored data'][language]}).click();
        await expect(page).toHaveURL(new RegExp(`/console/${op.path}\\?.*agent=cold-agent`));
        const section = page.locator('main section').filter({has:page.getByRole('heading', {name:op.label[language], exact:true})});
        await expect(section.locator('.op-badge')).toHaveText(`R0 · ${unavailable}`);
        await expect(section.locator('form[method="post"]')).toHaveCount(0);
        await page.getByRole('link', {name:['Sprawdź zapisane wyniki','Check stored results'][language]}).click();
        await expect(section.locator('.op-badge')).toHaveText(`R0 · ${unavailable}`);
        const alternate = lang === 'pl' ? 'en' : 'pl';
        await page.locator(`header a[hreflang="${alternate}"]`).click();
        await expect(page.locator('html')).toHaveAttribute('lang', alternate);
        await page.locator(`header a[hreflang="${lang}"]`).click();
        await expect(page.locator('html')).toHaveAttribute('lang', lang);
      }
    });

    await test.step('Every fresh query reports unavailable and enforces validation, CSRF and authorization', async () => {
      for (const op of operations) {
        const endpoint = `/console/${op.path}`;
        const form = {operation:op.operation, agent:'cold-agent', target:op.target || '', lang};
        const send = (updates = {}) => requestForm(page, endpoint, {...form, ...updates}, tokens.gm);
        expect((await send()).status(), op.operation).toBe(503);
        await csrfRejected(page, endpoint, form);
        expect((await requestForm(admin.page, endpoint, form, tokens.admin)).status(), op.operation).toBe(403);
        const anonymous = await requestForm(tokens.anonymousPage, endpoint, form, tokens.anonymous);
        expect(anonymous.status()).toBe(303);
        expect(anonymous.headers().location).toBe('/login');
        for (const invalid of [{agent:'bad/agent'}, {target:'bad/target'}, {lang:'invalid'}]) {
          expect((await send(invalid)).status(), `${op.operation} ${Object.keys(invalid)[0]}`).toBe(400);
        }
        expect((await send({operation:'unknown.operation'})).status()).toBe(422);
      }
    });

    await test.step('Search and open the cold player dossier, inventory, and sanction list', async () => {
      const endpoint = '/admin/console/players';
      expect((await page.goto(`${endpoint}?lang=${lang}`)).status()).toBe(200);
      const coldSections = page.locator('main section');
      await expect(coldSections.locator('li')).toHaveText([unavailable, unavailable, unavailable]);
      await expect(page.locator('main form[method="post"]')).toHaveCount(0);
      const search = {q:'Cold Player', lang};
      await csrfRejected(page, endpoint, search);
      expect((await requestForm(admin.page, endpoint, search, tokens.admin)).status()).toBe(403);
      const anonymous = await requestForm(tokens.anonymousPage, endpoint, search, tokens.anonymous);
      expect(anonymous.status()).toBe(303);
      expect(anonymous.headers().location).toBe('/login');
      expect((await requestForm(page, endpoint, {...search, q:'!'}, tokens.gm)).status()).toBe(400);
      expect((await requestForm(page, endpoint, {...search, lang:'invalid'}, tokens.gm)).status()).toBe(400);
      const result = await requestForm(page, endpoint, search, tokens.gm);
      expect(result.status()).toBe(303);
      expect(new URL(result.headers().location, page.url()).searchParams.get('q')).toBe(search.q);
      const coldSearch = await get(result.headers().location, {headers:{'sec-fetch-site':'same-origin'}});
      expect(coldSearch.status()).toBe(200);
      expect(await coldSearch.text()).toContain(unavailable);
      const dossier = page.locator('main form[method="get"]');
      await dossier.locator('[name="character"]').fill('cold-character');
      await dossier.getByRole('button').click();
      await expect(page).toHaveURL(/character=cold-character/);
      await expect(coldSections.locator('h2')).toHaveText(language === 0
        ? ['Postacie i dossier','Ekwipunek','Kary'] : ['Characters and dossier','Inventory','Sanctions']);
      await expect(coldSections.locator('li')).toHaveText([unavailable, unavailable, unavailable]);
      const invalid = await get(`${endpoint}?lang=${lang}&character=%21`, {headers:{'sec-fetch-site':'same-origin'}});
      expect(invalid.status()).toBe(400);
    });

    await test.step('Check role mutation audit rows and filter the cold ledger from the player console', async () => {
      await audit(admin.page, 'rbac.role.created', role);
      await audit(admin.page, 'rbac.role.granted', accountId);
      const changes = await query("SELECT before, after FROM audit_log WHERE action='rbac.role.granted' AND target=$1", [accountId]);
      expect(changes).toHaveLength(2);
      for (const change of changes) {
        expect(change.before).toEqual({role:change.after.role, assigned:false});
        expect(change.after.assigned).toBe(true);
      }
      expect(changes.some(change => change.after.role === role)).toBe(true);
      await page.locator('header a[href^="/admin/console/audit?"]').click();
      const form = page.locator('main form[method="get"]');
      await form.locator('[name="operation"]').fill('rbac.role.granted');
      await form.locator('[name="target"]').fill(accountId);
      await form.getByRole('button').click();
      await expect(page.locator('main section').first().getByText('rbac.role.granted', {exact:false})).toHaveCount(2);
      await expect(page.locator('main section').last().locator('li')).toHaveText(unavailable);
      expect((await get(`/admin/console/audit?lang=${lang}&request_id=bad`)).status()).toBe(400);
    });

    await test.step('Compare Game and Platform analytics access and cold data responses', async () => {
      const analyticsPage = admin.page;
      expect((await analyticsPage.goto('/admin/analytics')).status()).toBe(200);
      for (const section of analytics) {
        const path = `/admin/analytics${section}`;
        expect((await get(path)).status(), path).toBe(200);
        const gameData = await get(`${path}/data`);
        expect(gameData.status(), path).toBe(200);
        const gameBody = await gameData.json();
        expect(gameBody.state).toBe('unavailable');
        if (section === '/economy') expect(gameBody.groups).toHaveLength(5);
        await analyticsPage.locator(`main nav a[href="${path}"]`).click();
        await expect(analyticsPage.locator('html')).toHaveAttribute('lang', lang);
        await expect(analyticsPage.locator('main [data-state="unavailable"]')).toHaveCount(section === '/economy' ? 5 : 1);
        const data = await analyticsPage.request.get(`${path}/data`);
        expect(data.status()).toBe(200);
        expect((await data.json()).state).toBe(section === '/economy' ? 'available' : 'unavailable');
        expect(data.headers()['cache-control']).toBe('no-store, private');
        const filter = analyticsPage.locator('main form[method="get"]');
        await filter.locator('[name="world"]').fill('019b0760-0000-7000-8000-000000000001');
        // Reuse the server's valid default time window so it stays within retention.
        await filter.getByRole('button').click();
        await expect(analyticsPage).toHaveURL(/[?&]world=019b0760-0000-7000-8000-000000000001/);
        await expect(analyticsPage.locator('main [data-state="unavailable"]')).toHaveCount(section === '/economy' ? 5 : 1);
        if (section !== '/economy') {
          await expect(analyticsPage.locator('main [data-state="unavailable"]')).toContainText(unavailable);
        }
      }
    });
  } finally {
    await signedOut.close();
    await admin.close();
    if (gmId) await query('DELETE FROM account_roles WHERE account_id=$1', [gmId]);
  }
});
