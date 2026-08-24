import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import fs from 'node:fs';
import { expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  assertNoUnexpectedRuntimeFailures,
  attachRuntimeDiagnostics,
} from '../runtime-diagnostics.mjs';

export { allowExpectedHttpFailure, assertNoUnexpectedRuntimeFailures };

export const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
export const testedSha = process.env.ACCEPTANCE_SHA ?? 'local-unknown';
export const mailhogBaseUrl = process.env.ACCEPTANCE_MAILHOG_URL ?? 'http://127.0.0.1:8025';

function sanitizeUrl(rawUrl) {
  try {
    const url = new URL(rawUrl);
    url.search = '';
    url.hash = '';
    url.pathname = url.pathname.replace(/\/reset-password\/[^/]+/u, '/reset-password/[redacted]');
    return url.toString();
  } catch {
    return '[unparseable-url]';
  }
}

export function installDiagnostics(page) {
  const diagnostics = {
    testedSha,
    consoleErrors: [],
    pageErrors: [],
    failedRequests: [],
    httpErrors: [],
    serverErrors: [],
    expectedHttpFailures: [],
  };

  page.on('console', (message) => {
    if (message.type() === 'error') {
      diagnostics.consoleErrors.push({
        text: message.text().slice(0, 1000),
        url: sanitizeUrl(message.location().url ?? ''),
      });
    }
  });

  page.on('pageerror', (error) => {
    diagnostics.pageErrors.push({ message: error.message.slice(0, 1000) });
  });

  page.on('requestfailed', (request) => {
    diagnostics.failedRequests.push({
      method: request.method(),
      url: sanitizeUrl(request.url()),
      failure: request.failure()?.errorText ?? 'unknown',
    });
  });

  page.on('response', (response) => {
    if (response.status() >= 400) {
      diagnostics.httpErrors.push({
        status: response.status(),
        url: sanitizeUrl(response.url()),
      });
    }
    if (response.status() >= 500) {
      diagnostics.serverErrors.push({
        status: response.status(),
        url: sanitizeUrl(response.url()),
      });
    }
  });

  return diagnostics;
}

export async function attachDiagnostics(testInfo, diagnostics) {
  await attachRuntimeDiagnostics(testInfo, diagnostics, testedSha);
}

export function runPhpState(...args) {
  const output = execFileSync(
    'php',
    [path.join(repoRoot, 'scripts/acceptance/assert-platform-state.php'), ...args],
    {
      cwd: repoRoot,
      env: process.env,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );

  return JSON.parse(output.trim());
}

export function runArtisan(...args) {
  return execFileSync('php', ['artisan', ...args], {
    cwd: repoRoot,
    env: process.env,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

export function runBinary(binary, args) {
  return execFileSync(binary, args, {
    cwd: repoRoot,
    env: process.env,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

function base32Decode(value) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const normalized = value.toUpperCase().replace(/=+$/u, '').replace(/\s+/gu, '');
  let bits = '';

  for (const character of normalized) {
    const index = alphabet.indexOf(character);
    if (index < 0) {
      throw new Error('Invalid base32 MFA secret.');
    }
    bits += index.toString(2).padStart(5, '0');
  }

  const bytes = [];
  for (let offset = 0; offset + 8 <= bits.length; offset += 8) {
    bytes.push(Number.parseInt(bits.slice(offset, offset + 8), 2));
  }

  return Buffer.from(bytes);
}

export function totp(secret, timestampMs = Date.now()) {
  const counter = Math.floor(timestampMs / 1000 / 30);
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64BE(BigInt(counter));
  const digest = crypto.createHmac('sha1', base32Decode(secret)).update(buffer).digest();
  const offset = digest[digest.length - 1] & 0x0f;
  const binary = ((digest[offset] & 0x7f) << 24)
    | ((digest[offset + 1] & 0xff) << 16)
    | ((digest[offset + 2] & 0xff) << 8)
    | (digest[offset + 3] & 0xff);

  return String(binary % 1_000_000).padStart(6, '0');
}

export async function waitForDifferentTotp(secret, previousCode, timeoutMs = 35_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const code = totp(secret);
    if (code !== previousCode) {
      return code;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error('Timed out waiting for the next TOTP timestep.');
}

export function uniqueEmail(label) {
  const run = (process.env.ACCEPTANCE_RUN_ID ?? 'local').replace(/[^a-zA-Z0-9-]/gu, '-');
  const suffix = crypto.randomBytes(5).toString('hex');
  return `${label}+${run}-${suffix}@example.test`;
}

export function uniqueCharacterName(prefix = 'Test') {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz';
  const bytes = crypto.randomBytes(8);
  let suffix = '';
  for (const byte of bytes) {
    suffix += alphabet[byte % alphabet.length];
  }
  return `${prefix}${suffix}`.slice(0, 15);
}

function resetSerialRegistrationThrottle() {
  const isAccountProfile = process.env.ACCEPTANCE_RUN_SUFFIX === 'account';
  const isFullProfile = process.env.ACCEPTANCE_PROFILE === 'full';
  if (!isAccountProfile && !isFullProfile) {
    return;
  }

  // Account and full-profile specs share one synthetic HTTP source and execute
  // serially. Reset only the isolated acceptance cache before helper-driven
  // registration so a previous scenario cannot consume the next scenario's
  // allowance. Product limiters and within-scenario 429 assertions stay active.
  runArtisan('cache:clear');
}

export async function register(page, email, password) {
  resetSerialRegistrationThrottle();
  await page.goto('/register');
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByLabel('Confirm password').fill(password);
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page.getByRole('status')).toContainText('Registration completed.');
}

export async function login(page, email, password) {
  await page.goto('/login');
  const emailInput = page.locator('input[name="email"]');
  const passwordInput = page.locator('input[name="password"]');

  // Credential fields can re-settle while a repeated login form is restored,
  // especially in WebKit. Write identity first and the secret last, then use a
  // bounded field-stabilization loop; Playwright test retries remain zero.
  for (let attempt = 0; attempt < 3; attempt += 1) {
    await emailInput.fill(email);
    await passwordInput.fill(password);
    await page.waitForTimeout(50);

    if ((await emailInput.inputValue()) === email && (await passwordInput.inputValue()) === password) {
      break;
    }
  }
  await expect(emailInput).toHaveValue(email);
  await expect(passwordInput).toHaveValue(password);

  const loginResponse = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return response.request().method() === 'POST' && url.pathname === '/login';
  });
  await page.getByRole('button', { name: 'Sign in' }).click();
  await loginResponse;
}

export async function logout(page) {
  await page.goto('/mfa');
  const token = await page.locator('input[name="_token"]').first().inputValue();
  const status = await page.evaluate(async (csrfToken) => {
    const response = await fetch('/logout', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({ _token: csrfToken }),
    });
    return response.status;
  }, token);
  expect(status).toBe(200);
  await page.goto('/');
}

export async function enrollMfa(page, password) {
  await page.goto('/mfa');
  await page.getByRole('button', { name: 'Start MFA enrollment' }).click();
  const secret = (await page.locator('.secure-information code').first().textContent())?.trim();
  if (!secret) {
    throw new Error('MFA enrollment secret was not rendered.');
  }

  const enrollmentCode = totp(secret);
  await page.getByLabel('Current password').fill(password);
  await page.getByLabel('Six-digit authenticator code').fill(enrollmentCode);
  await page.getByRole('button', { name: 'Confirm and enable MFA' }).click();
  await expect(page.getByRole('heading', { name: 'Save your recovery codes now' })).toBeVisible();
  const recoveryCodes = await page.locator('main li code').allTextContents();
  if (recoveryCodes.length < 2) {
    throw new Error('MFA recovery codes were not rendered.');
  }

  return { secret, enrollmentCode, recoveryCodes: recoveryCodes.map((value) => value.trim()) };
}

export async function completeMfaChallenge(page, code, { expectSuccess = true } = {}) {
  const challengeHeading = page.getByRole('heading', { name: 'Complete your sign in' });
  await expect(challengeHeading).toBeVisible();
  await page.getByLabel('Authenticator or recovery code').fill(code);
  await page.getByRole('button', { name: 'Verify and sign in' }).click();
  if (expectSuccess) {
    await expect(challengeHeading).toBeHidden();
    await page.waitForLoadState('domcontentloaded');
  }
}

export async function waitForResetLink(email, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const response = await fetch(`${mailhogBaseUrl}/api/v2/messages`);
    if (response.ok) {
      const payload = await response.json();
      const normalized = JSON.stringify(payload)
        .replace(/=3D/gu, '=')
        .replace(/&amp;/gu, '&');
      if (normalized.includes(email)) {
        const match = normalized.match(/https?:\\?\/\\?\/[^\s"'<>]+\/reset-password\/[^\s"'<>\\]+(?:\?email=[^\s"'<>\\]+)?/u);
        if (match) {
          return match[0].replace(/\\\//gu, '/');
        }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error('Password reset message was not received by the test SMTP service.');
}

export async function assertAccessibilitySmoke(page) {
  const violations = await page.evaluate(() => {
    const findings = [];
    const controls = [...document.querySelectorAll('input:not([type="hidden"]), select, textarea')];
    for (const control of controls) {
      const labelled = control.labels?.length > 0
        || control.hasAttribute('aria-label')
        || control.hasAttribute('aria-labelledby');
      if (!labelled) findings.push(`unlabelled-control:${control.tagName.toLowerCase()}#${control.id || 'no-id'}`);
    }

    if (document.querySelectorAll('h1').length < 1) findings.push('missing-h1');

    for (const element of document.querySelectorAll('button, a[href]')) {
      const name = (element.textContent ?? '').trim()
        || element.getAttribute('aria-label')
        || element.getAttribute('title')
        || '';
      if (!name) findings.push(`unnamed-interactive:${element.tagName.toLowerCase()}`);
    }

    if (document.documentElement.scrollWidth > window.innerWidth + 1) {
      findings.push(`horizontal-overflow:${document.documentElement.scrollWidth}>${window.innerWidth}`);
    }

    return findings;
  });

  expect(violations, `Accessibility smoke violations on ${sanitizeUrl(page.url())}`).toEqual([]);

  await page.keyboard.press('Tab');
  const focus = await page.evaluate(() => {
    const element = document.activeElement;
    if (!(element instanceof HTMLElement) || element === document.body) return { focused: false, visible: false };
    const style = getComputedStyle(element);
    return {
      focused: true,
      visible: style.outlineStyle !== 'none' || style.outlineWidth !== '0px' || style.boxShadow !== 'none',
    };
  });
  expect(focus.focused).toBeTruthy();
  expect(focus.visible).toBeTruthy();
}

export async function evidenceScreenshot(page, name) {
  const directory = path.join(repoRoot, 'artifacts', 'acceptance', 'screenshots');
  fs.mkdirSync(directory, { recursive: true });
  // Preserve caret styling so screenshot collection stays compatible with strict style-src CSP.
  await page.screenshot({ path: path.join(directory, `${name}.png`), fullPage: true, caret: 'initial' });
}
