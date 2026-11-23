import { test, expect } from '@playwright/test';
import { appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { login, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

test('seeded account persists and restores a Privacy preference', async ({ page, baseURL }, info) => {
  const { lang } = info.project.metadata;
  const diagnostics = join(process.env.MONITOR_RUN_DIRECTORY, 'diagnostics');
  try {
    const target = new URL(baseURL);
    if (!['localhost', '127.0.0.1', '[::1]'].includes(target.hostname)) {
      throw new Error('Seeded Privacy checks require a loopback portal');
    }
    const { MONITOR_SEEDED_PRIVACY_EMAIL: email, MONITOR_SEEDED_PRIVACY_PASSWORD: password } = process.env;
    if (!email?.endsWith('@example.invalid') || !password) {
      throw new Error('Supply credentials from a disposable account seed');
    }
    const settings = async () => {
      const [row] = await query(
        'SELECT locale, public_account_association, public_status_visible, privacy_revision FROM accounts WHERE email=$1',
        [email],
      );
      return row;
    };
    const initial = await settings();
    expect(['pl', 'en']).toContain(initial?.locale);
    await login(page, lang, { email, password });

    const privacy = `/${lang}/account/privacy`;
    const locale = page.locator('main select[name="locale"]');
    const visibility = {
      public_account_association: initial.public_account_association,
      public_status_visible: initial.public_status_visible,
    };
    async function checkSettings(value) {
      await expect(locale).toHaveValue(value);
      for (const [name, checked] of Object.entries(visibility)) {
        await expect(page.locator(`main input[type="checkbox"][name="${name}"]`)).toBeChecked({ checked });
      }
    }
    async function saveLocale(value, revision) {
      await locale.selectOption(value);
      await submit(page, 'main form', visibility);
      await expect(page).toHaveURL(new URL(`/${value}/account/privacy`, target).href);
      expect((await page.goto(privacy)).status()).toBe(200);
      await checkSettings(value);
      expect(await settings()).toEqual({
        ...visibility,
        locale: value,
        privacy_revision: revision.toString(),
      });
    }

    expect((await page.goto(privacy)).status()).toBe(200);
    await checkSettings(initial.locale);
    const initialRevision = BigInt(initial.privacy_revision);
    let alternateVerified = false;
    try {
      await saveLocale(initial.locale === 'pl' ? 'en' : 'pl', initialRevision + 1n);
      alternateVerified = true;
    } finally {
      try {
        const observed = await settings();
        if (alternateVerified || observed.locale !== initial.locale ||
            Object.entries(visibility).some(([name, checked]) => observed[name] !== checked)) {
          expect((await page.goto(privacy)).status()).toBe(200);
          const restoredRevision = alternateVerified ? initialRevision + 2n : BigInt(observed.privacy_revision) + 1n;
          await saveLocale(initial.locale, restoredRevision);
        }
      } catch (restoreError) {
        if (alternateVerified) throw restoreError;
        appendFileSync(diagnostics, 'Diagnostic: Could not restore the seeded Privacy preference.\n');
      }
    }
    appendFileSync(diagnostics, `${info.project.name} passed\n`);
  } catch (error) {
    appendFileSync(diagnostics, `${info.project.name} failed at seeded Privacy write\n`);
    throw error;
  }
});
