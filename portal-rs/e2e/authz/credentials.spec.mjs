import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { password, seedPasswordRange, waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, seedBrowserAccount, sessionHash } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const prefixes = ['', '/pl', '/en'];
const loginCases = prefixes.flatMap(prefix => ['/login', '/mfa/login'].map(path => `${prefix}${path}`));
const passwordCases = prefixes.flatMap(prefix => ['POST', 'PUT'].map(method => [method, `${prefix}/password/change`]));
const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';

function checkRegistration(method, path, expected) {
  const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === path);
  expect(route.registrations[0].guard.kind, path).toBe(expected);
  return route.registrations[0].source;
}

async function waitForRegistrationBudget(name) {
  await expect.poll(async () => {
    const [row] = await query(`
      SELECT COALESCE(max(count),0) AS used FROM web_rate_limits
      WHERE limiter_name=$1 AND window_start=date_trunc('minute',clock_timestamp())
    `, [name]);
    return Number(row.used);
  }, { timeout: 65_000, intervals: [1000] }).toBeLessThan(5);
}

async function exerciseRegistration(request, locale, anonymous) {
  await query('UPDATE registration_abuse_policy SET enabled=true WHERE singleton AND NOT enabled');
  const token = await csrfPage(request, anonymous ? `/${locale}/login` : `/${locale}/`);
  for (const prefix of prefixes) {
    const challenge = `${prefix}/register/abuse/challenge`;
    checkRegistration('POST', challenge, 'anonymous');
    await test.step(`POST ${challenge}`, async () => {
      await waitForRegistrationBudget('identity-registration-challenge');
      const response = await request.post(challenge, {
        headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), 'crates/identity/src/abuse/http.rs:98').toBe(200);
        const body = await response.json();
        expect(Boolean(body.token), 'the server issued a work challenge').toBe(true);
        expect(body.bits).toBe(16);
      } finally {
        await response.dispose();
      }
    });

    const resend = `${prefix}/verify-email/resend`;
    checkRegistration('POST', resend, 'anonymous');
    await test.step(`POST ${resend}`, async () => {
      const { account, accountId } = seedBrowserAccount(locale);
      await query("UPDATE accounts SET email_verified_at=NULL,status='pending_verification' WHERE id=$1", [accountId]);
      await waitForRegistrationBudget('identity-registration');
      const response = await request.post(resend, {
        form: { email: account.email, _csrf: token },
        headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), 'crates/web/src/routes/verify.rs:293').toBe(303);
        expect(response.headers().location).toMatch(/^\/(pl|en)\/register\/check-inbox$/);
        const issued = await query(
          "SELECT 1 FROM account_email_tokens WHERE account_id=$1 AND purpose='verify_email'",
          [accountId],
        );
        expect(issued.length, 'only the anonymous resend selects the presented email').toBe(anonymous ? 1 : 0);
      } finally {
        await response.dispose();
      }
    });
  }
}

test.describe('browser login registrations', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, { project }) => {
      test.setTimeout(360_000);
      const { locale } = project.metadata;
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const anonymous = definition.key === 'anonymous';
      const existing = anonymous ? null : await openActor(locale, definition, issuer);
      try {
        for (const path of loginCases) {
          const source = checkRegistration('POST', path, 'anonymous');
          await test.step(`POST ${path}`, async () => {
            const { request } = existing ?? await openActor(locale, definition, issuer);
            const login = existing ?? seedBrowserAccount(locale);
            try {
              const token = await csrfPage(request, anonymous ? `/${locale}/login` : `/${locale}/`);
              await waitForLoginBudget();
              const response = await request.post(path, {
                form: { ...login.account, _csrf: token }, maxRedirects: 0,
              });
              try {
                expect(response.status(), source).toBe(302);
                expect(response.headers().location).toMatch(/^\/(pl|en)\/$/);
                const [session] = await query('SELECT account_id FROM web_sessions WHERE id_hash=$1', [await sessionHash(request)]);
                expect(session?.account_id, 'successful login keeps the expected identity').toBe(login.accountId);
              } finally {
                await response.dispose();
              }
            } finally {
              if (anonymous) await request.dispose();
            }
          });
        }
        const { request } = existing ?? await openActor(locale, definition, issuer);
        try {
          await exerciseRegistration(request, locale, anonymous);
        } finally {
          if (anonymous) await request.dispose();
        }
      } finally {
        await existing?.request.dispose();
      }
    });
  }
});

test.describe('authenticated password changes', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, { project }) => {
      test.setTimeout(360_000);
      const { locale } = project.metadata;
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const anonymous = definition.key === 'anonymous';
      const replacement = password();
      if (!anonymous) await seedPasswordRange(replacement);
      const db = database();
      try {
        for (const [method, path] of passwordCases) {
          checkRegistration(method, path, 'logged-in');
          await test.step(`${method} ${path}`, async () => {
            const { request, account, accountId } = await openActor(locale, definition, issuer);
            try {
              const token = await csrfPage(request, anonymous ? `/${locale}/login` : `/${locale}/`);
              const hash = await sessionHash(request);
              const before = anonymous ? await db.snapshot(hash) : null;
              const stored = async () => (await query(
                "SELECT encode(sha256(convert_to(password_hash,'UTF8')),'hex') AS fingerprint FROM accounts WHERE id=$1",
                [accountId],
              ))[0].fingerprint;
              const previous = anonymous ? null : await stored();
              const response = await request.fetch(path, {
                method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
                form: { current_password: account?.password ?? 'unused', password: replacement, password_confirmation: replacement },
              });
              try {
                expect(response.status(), 'crates/identity/src/credentials_http.rs:260').toBe(anonymous ? 401 : 302);
                if (before) {
                  assertRefusalState(before, await db.snapshot(hash));
                } else {
                  expect(response.headers().location).toBe('/login');
                  expect((await stored()) !== previous, 'stored password was changed').toBe(true);
                  expect((await query('SELECT 1 FROM web_sessions WHERE id_hash=$1', [hash])).length).toBe(0);
                }
              } finally {
                await response.dispose();
              }
            } finally {
              await request.dispose();
            }
          });
        }
      } finally {
        await db.close();
      }
    });
  }
});
