import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

async function send() {
  const { MONITOR_SMTP_HOST: host, MONITOR_SMTP_USER: user, MONITOR_SMTP_PASSWORD: password,
    MONITOR_TRANSITION: transition, GITHUB_SERVER_URL: server, GITHUB_REPOSITORY: repo,
    GITHUB_RUN_ID: run } = process.env;
  const smtp = new URL(host.includes('://') ? host : `smtp://${host}`);
  if (!['smtp:', 'smtps:'].includes(smtp.protocol) || smtp.username || smtp.password ||
      !user || /[:\x00-\x1f]/.test(user) ||
      !['failing', 'recovered'].includes(transition)) {
    throw new Error('Invalid mail configuration');
  }
  smtp.port ||= smtp.protocol === 'smtp:' ? '587' : '465';

  const to = 'support@oteryn.com';
  const sender = /^[^\s<>@]+@[^\s<>@]+$/.test(user) ? user : to;
  const mail = [
    `From: ${sender}`, `To: ${to}`, `Date: ${new Date().toUTCString()}`,
    `Subject: Production synthetic monitor ${transition}`, '',
    `Run: ${server}/${repo}/actions/runs/${run}`, '',
  ].join('\r\n');
  // Keep SMTP credentials out of the process arguments and curl diagnostics.
  const quote = (value) => '"' + value.replace(/[\\"\r\n\t]/g, (char) =>
    ({ '\\': '\\\\', '"': '\\"', '\r': '\\r', '\n': '\\n', '\t': '\\t' })[char]) + '"';
  const scratch = await mkdtemp(join(tmpdir(), 'prod-monitor-mail-'));
  try {
    const message = join(scratch, 'message');
    await writeFile(message, mail, { mode: 0o600 });
    const client = spawn('curl', ['--disable', '--silent', '--ssl-reqd', '--max-time', '25',
      '--config', '-', '--upload-file', message, '--mail-rcpt', to],
    { stdio: ['pipe', 'ignore', 'ignore'], env: { PATH: process.env.PATH } });
    client.stdin.on('error', () => {});
    client.stdin.end(`url = ${quote(smtp.href)}\nuser = ${quote(`${user}:${password}`)}\nmail-from = ${quote(sender)}\n`);
    const [code] = await once(client, 'close');
    if (code !== 0) throw new Error('SMTP delivery failed');
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}

try {
  await send();
} catch {
  console.error('Production monitor email delivery failed. Check the SMTP configuration and relay.');
  process.exitCode = 1;
}
