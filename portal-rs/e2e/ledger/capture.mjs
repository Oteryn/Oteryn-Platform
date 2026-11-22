import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';

const cwd = new URL('.', import.meta.url);
const git = (...args) => execFileSync('git', args, {cwd, encoding: 'utf8'}).trim();

export function inventoryDigest() {
  return createHash('sha256')
    .update(readFileSync(new URL('../inventory.json', cwd)))
    .update('\0')
    .update(readFileSync(new URL('forms.json', cwd)))
    .digest('hex');
}

export function beginCapture() {
  return {
    revision: git('rev-parse', 'HEAD'),
    runId: randomUUID(),
    clean: git('status', '--porcelain', '--untracked-files=normal') === '',
    inventory: inventoryDigest(),
  };
}

export function validateCapture(capture) {
  if (!/^[a-f0-9]{40}$/.test(capture?.revision) ||
      !/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(capture?.runId) ||
      !/^[a-f0-9]{64}$/.test(capture?.inventory)) {
    throw new Error('Missing or invalid ledger capture provenance. Rerun Playwright.');
  }
  if (capture.clean !== true) {
    throw new Error('Ledger evidence requires a clean checkout at capture time.');
  }
  return capture;
}

export function sameCapture(left, right) {
  return left?.revision === right.revision && left.runId === right.runId &&
    left.clean === right.clean && left.inventory === right.inventory;
}
