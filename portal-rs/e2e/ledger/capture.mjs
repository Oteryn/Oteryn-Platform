import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';

const cwd = new URL('.', import.meta.url);
const git = (...args) => execFileSync('git', args, {cwd, encoding: 'utf8'}).trim();
const mutations = new URL('../mutation/forms.json', cwd);
const routeLocale = route => route.replaceAll('{locale}', '{lang}');

export function ledgerInventory() {
  const inventory = JSON.parse(readFileSync(new URL('../inventory.json', cwd)));
  const { routes } = JSON.parse(readFileSync(mutations));
  for (const { route, method, sources } of routes) {
    const registrations = inventory.routes.filter(row => routeLocale(row.route) === routeLocale(route));
    if (registrations.some(row => row.methods.includes(method))) continue;
    const [, crate] = sources[0].source.split('/');
    const owner = registrations.find(row => row.crate === crate);
    if (owner) {
      owner.methods.push(method);
    } else {
      inventory.routes.push({ route, crate, methods: [method], sources: sources.map(row => row.source) });
    }
  }
  return inventory;
}

export function inventoryDigest() {
  return createHash('sha256')
    .update(readFileSync(new URL('../inventory.json', cwd)))
    .update('\0')
    .update(readFileSync(new URL('forms.json', cwd)))
    .update('\0')
    .update(readFileSync(mutations))
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
