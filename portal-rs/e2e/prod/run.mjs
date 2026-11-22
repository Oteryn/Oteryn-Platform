import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

async function run() {
  // Node enables its own debug logger before loading this launcher.
  if (process.env.NODE_DEBUG) {
    console.error('Unset NODE_DEBUG before running the production monitor.');
    return 1;
  }
  const args = process.argv.slice(2);
  if (args.length > 1 || args.some((arg) => !/^--project=(pl-1440|en-390)$/.test(arg))) {
    console.error('Usage: node prod/run.mjs [--project=pl-1440|--project=en-390]');
    return 1;
  }

  const dir = await mkdtemp(join(tmpdir(), 'portal-prod-monitor-'));
  try {
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
      !/^(DEBUG|NODE_DEBUG|NODE_OPTIONS$|PW|PLAYWRIGHT_)/i.test(key)
      || key === 'PLAYWRIGHT_BROWSERS_PATH',
    ));
    env.PLAYWRIGHT_NO_COPY_PROMPT = '1';
    env.MONITOR_RUN_DIRECTORY = dir;
    const counter = join(dir, 'requests');
    await writeFile(counter, '0');
    const cli = fileURLToPath(new URL('../node_modules/@playwright/test/cli.js', import.meta.url));
    const config = fileURLToPath(new URL('playwright.prod.config.mjs', import.meta.url));
    const child = spawn(process.execPath, [cli, 'test', '--config', config, ...args], {
      env, stdio: 'ignore',
    });
    const [code] = await once(child, 'close');
    const diagnostics = await readFile(join(dir, 'diagnostics'), 'utf8').catch(() => '');
    for (const line of diagnostics.split('\n')) {
      if (/^(pl-1440|en-390) (passed|failed at [A-Za-z0-9 _-]+)$/.test(line)
          || /^Diagnostic: [A-Za-z0-9 /_:(),.-]{1,160}$/.test(line)) console.log(line);
    }
    const count = await readFile(counter, 'utf8');
    if (/^\d{1,2}$/.test(count)) console.log(`Monitor sent ${count} requests.`);
    if (code !== 0) console.error('Production monitor failed.');
    return code === 0 ? 0 : 1;
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

try {
  process.exitCode = await run();
} catch {
  console.error('Production monitor launcher failed.');
  process.exitCode = 1;
}
