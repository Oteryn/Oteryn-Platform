import { createHash } from 'node:crypto';
import { appendFileSync, mkdirSync, readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from '@playwright/test';
import { createRequire } from 'node:module';
import { pool } from '../helpers.mjs';

export const inventory = JSON.parse(readFileSync(new URL('../inventory.json', import.meta.url)));
export const forms = JSON.parse(readFileSync(new URL('./forms.json', import.meta.url)));

const parameter = /\{([^{}]+)\}/g;

function matcher(pattern) {
  const parts = pattern.split(parameter);
  const expression = parts.map((part, i) => {
    if (i % 2 === 0) return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    if (part === 'lang' || part === 'locale') return '(?:pl|en)';
    return part.startsWith('*') ? '.+' : '[^/]+';
  }).join('');
  return new RegExp(`^${expression}$`);
}

function precedence(pattern) {
  return pattern.split('/').map(part => {
    if (!part.includes('{')) return '3';
    if (part === '{lang}' || part === '{locale}') return '2';
    return part.includes('{*') ? '0' : '1';
  }).join('');
}

function bySpecificity(a, b) {
  return precedence(b).localeCompare(precedence(a));
}

const routes = inventory.routes.map(row => row.route).sort(bySpecificity)
  .map(pattern => ({ pattern, match: matcher(pattern) }));
const destinations = forms.map(form => ({ ...form, match: matcher(form.action) }))
  .sort((a, b) => bySpecificity(a.action, b.action));

// Extract only field names and compare their values to the static submitter
// allowlist. Never serialize the request URL, payload, headers or cookie.
function submittedFields(url, method, contentType, body) {
  if (method === 'GET') return url.searchParams;
  if (contentType.startsWith('application/x-www-form-urlencoded')) {
    return new URLSearchParams(body?.toString());
  }
  if (contentType.startsWith('multipart/form-data') && body) {
    const boundary = /boundary=(?:"([^"]+)"|([^;\s]+))/i.exec(contentType);
    const parts = boundary ? body.toString('latin1').split(`--${boundary[1] ?? boundary[2]}`) : [];
    const fields = new URLSearchParams();
    for (const part of parts) {
      const separator = part.indexOf('\r\n\r\n');
      if (separator < 0) continue;
      const disposition = /^content-disposition: form-data;[^\r\n]*\bname="([^"]+)"([^\r\n]*)/im
        .exec(part.slice(0, separator));
      if (disposition && !/\bfilename=/i.test(disposition[2])) {
        fields.append(disposition[1], Buffer.from(part.slice(separator + 4, -2), 'latin1').toString());
      }
    }
    return fields;
  }
  return null;
}

function matchesButton(button, value, url, referer) {
  const offset = /^{{ page ([+-]) 1 }}$/.exec(button.value);
  if (!offset) return value === button.value;
  if (!referer || !/^\d+$/.test(value)) return false;
  const previous = new URL(referer);
  if (previous.origin !== url.origin || previous.pathname !== url.pathname) return false;
  const page = Number(previous.searchParams.get('page') ?? 1);
  return Number(value) === page + (offset[1] === '+' ? 1 : -1);
}

function formAction(url, method, contentType, body, referer) {
  const candidates = destinations.filter(form => form.method === method && form.match.test(url.pathname));
  if (!candidates.length) return null;
  const fields = submittedFields(url, method, contentType, body);
  if (!fields || !Array.from(fields).length) return null;

  const action = candidates[0].action;
  const matching = candidates.filter(form => form.action === action);
  const buttons = matching.flatMap(form => form.buttons).filter(Boolean);
  const selected = buttons.filter(button => fields.getAll(button.name).some(value =>
    matchesButton(button, value, url, referer)));
  if (selected.length) {
    return { action, buttons: [...new Map(selected.map(button => [JSON.stringify(button), button])).values()] };
  }
  // A GET navigation with unrelated query parameters is not form evidence.
  // Fields that can also be submitter names must match a declared value.
  if (matching.some(form => form.buttons.includes(null) &&
      form.fields.some(name => fields.has(name) && !buttons.some(button => button.name === name)))) {
    return { action, buttons: [null] };
  }
  return null;
}

const directory = process.env.PORTAL_E2E_LEDGER_HITS ??
  fileURLToPath(new URL('../test-results/ledger/', import.meta.url));
const testDirectory = fileURLToPath(new URL('../', import.meta.url));
const sessionCookie = /(?:^|;\s*)__Host-oteryn_session=([^;]+)/;
const pending = new Set();
let database;

async function actorFor(cookie) {
  const token = sessionCookie.exec(cookie ?? '')?.[1];
  if (!token) return 'anonymous';
  if (!database) {
    database = pool();
    database.options.allowExitOnIdle = true;
    database.options.max = 2;
  }
  const hash = createHash('sha256').update(token).digest();
  const { rows } = await database.query(`
    SELECT ARRAY(SELECT r.key FROM account_roles ar JOIN roles r ON r.id = ar.role_id
                 WHERE ar.account_id = s.account_id ORDER BY r.key) AS roles
    FROM web_sessions s JOIN accounts a ON a.id = s.account_id
    WHERE s.id_hash = $1 AND s.idle_expires_at > clock_timestamp()
      AND s.absolute_expires_at > clock_timestamp()
  `, [hash]);
  if (!rows.length) return 'anonymous';
  return rows[0].roles.length ? rows[0].roles : ['player'];
}

function start(url, method, contentType, body, cookie, referer) {
  let info;
  try {
    info = test.info();
  } catch {
    return null;
  }
  const target = new URL(url);
  const origin = new URL(info.project.use.baseURL).origin;
  const local = target.origin === origin;
  const row = {
    route_pattern: local ? routes.find(route => route.match.test(target.pathname))?.pattern ?? null : null,
    method,
    status: null,
    actor: 'anonymous',
    form_action: local ? formAction(target, method, contentType, body, referer) : null,
    spec: relative(testDirectory, info.file),
    test: info.titlePath.slice(1).join(' › '),
    project: info.project.name,
  };
  const request = {
    file: resolve(directory, `${row.project}-${info.workerIndex}.jsonl`),
    row,
    actor: local ? Promise.resolve(cookie).then(actorFor).then(actor => { row.actor = actor; }) : Promise.resolve(),
  };
  // Surface lookup failures without leaking connection parameters in an error.
  request.actor = request.actor.catch(() => {
    row.actor = null;
    process.exitCode = 1;
  });
  pending.add(request);
  return request;
}

function write(request) {
  if (!pending.delete(request)) return;
  mkdirSync(directory, { recursive: true });
  appendFileSync(request.file, JSON.stringify(request.row) + '\n');
}

function finish(request, status) {
  if (!request) return;
  request.row.status = status;
  void request.actor.then(() => write(request));
}

export function installRecorder() {
  // Resolve the runner's copy. Other dependencies can install a separate
  // playwright-core version alongside it.
  const require = createRequire(import.meta.url);
  const playwright = createRequire(require.resolve('playwright'))('playwright-core');
  const browserRequests = new WeakMap();
  const apiRequests = new WeakMap();
  const watchApi = context => {
    // Playwright emits server events for each redirect hop, including the
    // intermediate status and the cookies actually sent on that hop.
    const implementation = context._connection.toImpl(context);
    implementation.on('request', event => {
      const { url, method, headers, postData } = event;
      const header = name => Object.entries(headers).find(([key]) => key.toLowerCase() === name)?.[1];
      apiRequests.set(event, start(url, method, header('content-type') ?? '', postData, header('cookie'), header('referer')));
    });
    implementation.on('requestfinished', event => finish(apiRequests.get(event.requestEvent), event.statusCode));
  };
  playwright._instrumentation.addListener({
    runAfterCreateBrowserContext(context) {
      watchApi(context.request);
      context.on('request', request => {
        const headers = request.headers();
        const cookie = request.headerValue('cookie').catch(() => null);
        browserRequests.set(request, start(request.url(), request.method(),
          headers['content-type'] ?? '', request.postDataBuffer(), cookie, headers.referer));
      });
      context.on('response', response => finish(browserRequests.get(response.request()), response.status()));
      context.on('requestfailed', request => {
        const hit = browserRequests.get(request);
        if (hit?.row.status === null) finish(hit, null);
      });
    },
    runAfterCreateRequestContext: watchApi,
  });
  // Terminated requests can lack a response event, including API transport
  // errors. Record those attempts with a null status.
  process.once('exit', () => {
    for (const request of pending) write(request);
  });
}
