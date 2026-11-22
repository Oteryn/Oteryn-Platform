import { randomUUID } from 'node:crypto';
import { expect, test } from '../fixtures.mjs';
import { query, rolePage } from '../staff-rbac-helpers.mjs';
import { expectRedirect, parallelSubmit, reloadGet, signIn, snapshot, submitForm } from './forms.mjs';

const roles = '/admin/roles';
const create = 'main form:has(button[value=create])';
const edit = 'main form:has(button[value=set-permissions])';
const copy = {
  en: {
    heading: 'Staff panel',
    duplicate: 'A role with this key already exists.',
    replacement: 'Replacement permissions (separated by spaces)',
  },
  pl: {
    heading: 'Panel zespołu',
    duplicate: 'Rola o tym kluczu już istnieje.',
    replacement: 'Nowy zestaw uprawnień (oddzielone spacjami)',
  },
};

function permissions(id) {
  return query('SELECT permission FROM role_permissions WHERE role_id=$1 ORDER BY permission', [id]);
}

function audits(id, action) {
  return query(
    'SELECT actor, before, after FROM audit_log WHERE target=$1 AND action=$2 ORDER BY created_at, id',
    [id, action],
  );
}

test('role creation has one effect and two tabs apply explicit, audited replacements', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const actor = await signIn(page, lang, 'platform_admin');
  expect((await page.goto(roles)).status()).toBe(200);
  const key = `double_submit_${randomUUID().replaceAll('-', '')}`;
  const createForm = page.locator(create);
  await createForm.locator('[name=key]').fill(key);
  await createForm.locator('[name=permissions]').fill('admin.access');
  const createButton = createForm.locator('button[value=create]');
  const original = await snapshot(createButton);
  const [accepted, refused] = (await parallelSubmit(page, createButton, original))
    .sort((a, b) => a.status() - b.status());
  expectRedirect(accepted, 303, roles);
  expect(refused.status()).toBe(409);
  expect(await refused.text()).toBe(copy[lang].duplicate);
  const rows = await query('SELECT id FROM roles WHERE key=$1', [key]);
  expect(rows).toEqual([{ id: expect.any(String) }]);
  const [{ id }] = rows;
  expect(await permissions(id)).toEqual([{ permission: 'admin.access' }]);
  const creation = [{ actor: actor.id, before: null, after: { key, permissions: ['admin.access'] } }];
  expect(await audits(id, 'rbac.role.created')).toEqual(creation);

  const second = await context.newPage();
  const firstPermissions = ['admin.access', 'audit.view'];
  const secondPermissions = ['admin.access', 'cms.pages.manage'];
  for (const [tab, replacement] of [[page, firstPermissions], [second, secondPermissions]]) {
    await rolePage(tab, id);
    await expect(tab.locator('html')).toHaveAttribute('lang', lang);
    await expect(tab.getByRole('heading', { level: 1 })).toHaveText(copy[lang].heading);
    const form = tab.locator(edit);
    await form.locator('select[name=role]').selectOption(id);
    await form.getByLabel(copy[lang].replacement).fill(replacement.join(' '));
  }
  const firstButton = page.locator(`${edit} button[value=set-permissions]`);
  const first = await snapshot(firstButton);
  for (const response of await parallelSubmit(page, firstButton, first)) {
    expectRedirect(response, 303, roles);
  }
  await expect(page).toHaveURL(roles);
  await reloadGet(page);
  expect(await permissions(id)).toEqual(firstPermissions.map(permission => ({ permission })));
  const firstAudit = {
    actor: actor.id, before: { permissions: ['admin.access'] }, after: { permissions: firstPermissions },
  };
  expect(await audits(id, 'rbac.role.permissions_changed')).toEqual([firstAudit]);

  const secondButton = second.locator(`${edit} button[value=set-permissions]`);
  const replacement = await snapshot(secondButton);
  expectRedirect(await submitForm(second, secondButton, replacement.url), 303, roles);
  await expect(second).toHaveURL(roles);
  await reloadGet(second);
  await expect(second.getByRole('heading', { level: 1 })).toHaveText(copy[lang].heading);
  expect(await permissions(id)).toEqual(secondPermissions.map(permission => ({ permission })));
  expect(await audits(id, 'rbac.role.permissions_changed')).toEqual([
    firstAudit,
    { actor: actor.id, before: { permissions: firstPermissions }, after: { permissions: secondPermissions } },
  ]);
  expect(await audits(id, 'rbac.role.created')).toEqual(creation);
});
