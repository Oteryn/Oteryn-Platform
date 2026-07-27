import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import fs from 'node:fs';
import { expect } from '@playwright/test';

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
    serverErrors: [],
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
  await testInfo.attach('exact-tested-sha', {
    body: Buffer.from(`${testedSha}\n`, 'utf8'),
    contentType: 'text/plain',
  });
  await testInfo.attach('browser-diagnostics', {
    body: Buffer.from(JSON.stringify(diagnostics, null, 2), 'utf8'),
    contentType: 'application/json',
  });
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

function resetAccountProfileRegistrationThrottle() {
  if (process.env.ACCEPTANCE_RUN_SUFFIX !== 'account') {
    return;
  }

  // Account-profile specs intentionally share one HTTP source and run serially.
  // Clearing the isolated acceptance cache here prevents one scenario's
  // registration attempts from consuming the next scenario's source limiter.
  // Product limiters remain enabled and within-scenario 429 assertions remain valid.
  runArtisan('cache:clear');
}

export async function register(page, email, password) {
  resetAccountProfileRegistrationThrottle();
  await page.goto('/register');
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByLabel('Confirm password').fill(password);
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page.getByRole('status')).toContainText('Registration completed.');
}

export async function login(page, email, password) {
  await page.goto('/login');
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password').fill(password);
  await page.getByRole('button', { name: 'Sign in' }).click();
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
  await page.getByLabel('Authenticator code').fill(enrollmentCode);
  await page.getByRole('button', { name: 'Confirm MFA' }).click();
  await expect(page.getByRole('status')).toContainText('Multi-factor authentication enabled.');

  const recoveryCodes = await page.locator('.recovery-code-list code').allTextContents();
  if (recoveryCodes.length === 0) {
    throw new Error('MFA recovery codes were not rendered.');
  }

  return { secret, enrollmentCode, recoveryCodes: recoveryCodes.map((code) => code.trim()) };
}

export async function completeMfaChallenge(page, code) {
  await page.getByLabel('Authentication code').fill(code);
  await page.getByRole('button', { name: 'Verify' }).click();
}
