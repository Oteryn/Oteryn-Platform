import { test, expect } from '@playwright/test';
import { appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { csrfRejected, waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { guardPrivacyOrigin, privacyFormAction } from './privacy-origin.mjs';

test('seeded account persists and restores a Privacy preference', async ({ page, baseURL }, info) => {
  const { lang } = info.project.metadata;
  const diagnostics = join(process.env.MONITOR_RUN_DIRECTORY, 'diagnostics');
  try {
    const target = new URL(baseURL);
    if (!['http:', 'https:'].includes(target.protocol) ||
        !['localhost', '127.0.0.1', '[::1]'].includes(target.hostname) || target.username || target.password) {
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
    const session = await page.context().newCDPSession(page);
    const verifyOrigin = await guardPrivacyOrigin(page, session, target.origin);
    async function visit(path) {
      const response = await page.goto(path);
      verifyOrigin();
      expect(response.status()).toBe(200);
    }
    async function submitLocal(values) {
      verifyOrigin();
      const form = page.locator('main form').first();
      const action = privacyFormAction(page.url(), await form.evaluate(form => form.action), target.origin);
      const fields = await form.evaluate(form => Object.fromEntries(new FormData(form)));
      verifyOrigin();
      await csrfRejected(page, action, { ...fields, ...values });
      for (const [name, value] of Object.entries(values)) {
        const input = form.locator(`[name="${name}"]`);
        if (typeof value === 'boolean') await input.setChecked(value);
        else await input.fill(value);
      }
      verifyOrigin();
      await Promise.all([
        page.waitForNavigation(),
        form.locator('button[type=submit], button:not([type])').first().click(),
      ]);
      verifyOrigin();
    }
    await visit(`/${lang}/login`);
    await waitForLoginBudget();
    await submitLocal({ email, password });

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
      verifyOrigin();
      await locale.selectOption(value);
      await submitLocal(visibility);
      await expect(page).toHaveURL(new URL(`/${value}/account/privacy`, target).href);
      await visit(privacy);
      await checkSettings(value);
      expect(await settings()).toEqual({
        ...visibility,
        locale: value,
        privacy_revision: revision.toString(),
      });
    }

    await visit(privacy);
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
          await visit(privacy);
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
