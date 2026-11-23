import { createHmac } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { atPath, expect, submit, test, visit } from './fixtures.mjs';
import { copy, login, logout, verifiedLogin } from './identity.mjs';

const stepMs = 30_000;

function decodeBase32(secret) {
  const normalized = secret.toUpperCase().replace(/=+$/, '');
  expect(/^[A-Z2-7]+$/.test(normalized), 'Provisioning displays a Base32 authenticator secret').toBe(true);
  const bytes = [];
  let bits = 0;
  let value = 0;
  for (const character of normalized) {
    value = (value << 5) | 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'.indexOf(character);
    bits += 5;
    if (bits >= 8) {
      bytes.push((value >>> (bits - 8)) & 255);
      bits -= 8;
    }
  }
  return Buffer.from(bytes);
}

function totp(key, step) {
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(step));
  const digest = createHmac('sha1', key).update(counter).digest();
  const offset = digest.at(-1) & 15;
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

async function usableStep(after = 0) {
  const now = Date.now();
  const step = Math.floor(now / stepMs);
  if (step > after && now % stepMs < 25_000) return step;
  const next = Math.max(step, after) + 1;
  await delay(next * stepMs - now + 500);
  return Math.floor(Date.now() / stepMs);
}

async function enroll(page, lang, account, mail) {
  await verifiedLogin(page, lang, account, mail);
  await visit(page, `/${lang}/mfa`);
  await submit(page, `/${lang}/mfa/enroll`, 200, { password: account.password });
  const key = decodeBase32((await page.locator('main .op-prose').innerText()).trim());
  const step = await usableStep();
  await submit(page, `/${lang}/mfa/confirm`, 200, { password: account.password, code: totp(key, step) });
  const codes = page.locator('textarea[readonly]');
  await expect(codes.first(), 'MFA confirmation displays recovery codes').toBeVisible();
  return { key, step, recovery: (await codes.first().inputValue()).trim() };
}

async function challenge(page, lang, code, accepted) {
  const path = `/${lang}/mfa/challenge`;
  await submit(page, path, 302, { code }, 'main form[action$="/mfa/challenge"]');
  await atPath(page, accepted ? `/${lang}/` : path);
  if (accepted) {
    await visit(page, `/${lang}/account`);
  } else {
    await expect(page.locator('main [role=alert]'), `POST ${path} generic refusal`).toHaveText(copy[lang].loginError);
  }
}

test('4.1 TOTP enrollment and an authenticator challenge grant account access', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  const { key, step } = await enroll(page, lang, account, mail);
  await login(page, lang, account, 'mfa/challenge');
  await challenge(page, lang, totp(key, await usableStep(step)), true);
});

test('4.2 A wrong authenticator code is refused', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  const { key } = await enroll(page, lang, account, mail);
  await login(page, lang, account, 'mfa/challenge');
  const step = await usableStep();
  const nearby = new Set([-2, -1, 0, 1, 2].map(offset => totp(key, step + offset)));
  const wrong = ['000000', '111111', '222222', '333333', '444444', '555555'].find(code => !nearby.has(code));
  await challenge(page, lang, wrong, false);
});

test('4.3 A recovery code grants access once and fails on reuse', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  const { recovery } = await enroll(page, lang, account, mail);
  await login(page, lang, account, 'mfa/challenge');
  await challenge(page, lang, recovery, true);
  await logout(page, lang);
  await login(page, lang, account, 'mfa/challenge');
  await challenge(page, lang, recovery, false);
});
