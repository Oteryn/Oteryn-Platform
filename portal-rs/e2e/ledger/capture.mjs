import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';

const cwd = new URL('.', import.meta.url);
const git = (...args) => execFileSync('git', args, {cwd, encoding: 'utf8'}).trim();

export function beginCapture() {
  return {
    revision: git('rev-parse', 'HEAD'),
    runId: randomUUID(),
    clean: git('status', '--porcelain', '--untracked-files=normal') === '',
  };
}

export function validateCapture(capture) {
  if (!/^[a-f0-9]{40}$/.test(capture?.revision) ||
      !/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(capture?.runId)) {
    throw new Error('Missing or invalid ledger capture provenance. Rerun Playwright.');
  }
  if (capture.clean !== true) {
    throw new Error('Ledger evidence requires a clean checkout at capture time.');
  }
  return capture;
}

export function sameCapture(left, right) {
  return left?.revision === right.revision && left.runId === right.runId && left.clean === right.clean;
}
