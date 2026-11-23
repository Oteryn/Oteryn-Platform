import { createHash } from 'node:crypto';
import { appendFileSync, mkdirSync, readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from '@playwright/test';
import { createRequire } from 'node:module';
import { pool } from '../helpers.mjs';
import { ledgerInventory } from './capture.mjs';

export const inventory = ledgerInventory();
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
  const submitted = matching.filter(form => {
    if (!form.operation) return true;
    const values = fields.getAll(form.operation.name);
    return values.length === 1 && values[0] === form.operation.value;
  });
  const buttons = submitted.flatMap(form => form.buttons).filter(Boolean);
  const selected = buttons.filter(button => fields.getAll(button.name).some(value =>
    matchesButton(button, value, url, referer)));
  if (selected.length) {
    return { action, buttons: [...new Map(selected.map(button => [JSON.stringify(button), button])).values()] };
  }
  // Fields that can also be submitter names must match a declared value.
  const unnamed = submitted.find(form => form.buttons.includes(null) &&
      form.fields.some(name => fields.has(name) && !buttons.some(button => button.name === name)));
  if (unnamed) {
    return { action, buttons: [null], ...(unnamed.operation && { operation: unnamed.operation }) };
  }
  return null;
}

const directory = process.env.PORTAL_E2E_LEDGER_HITS ??
  fileURLToPath(new URL('../test-results/ledger/', import.meta.url));
const testDirectory = fileURLToPath(new URL('../', import.meta.url));
const sessionCookie = /(?:^|;\s*)__Host-oteryn_session=([^;]+)/;
const attempts = new WeakMap();
let database;

async function sessionActor(token) {
  if (!database) {
    database = pool();
    database.options.allowExitOnIdle = true;
    database.options.max = 2;
  }
  const hash = createHash('sha256').update(token).digest();
  const { rows } = await database.query(`
    SELECT s.account_id, ARRAY(SELECT r.key FROM account_roles ar JOIN roles r ON r.id = ar.role_id
                 WHERE ar.account_id = s.account_id ORDER BY r.key) AS roles
    FROM web_sessions s
    WHERE s.id_hash = $1 AND s.idle_expires_at > clock_timestamp()
      AND s.absolute_expires_at > clock_timestamp()
  `, [hash]);
  if (!rows.length) return null;
  if (!rows[0].account_id) return 'anonymous';
  return rows[0].roles.length ? rows[0].roles : ['player'];
}

function currentAttempt() {
  let info;
  try {
    info = test.info();
  } catch {
    return null;
  }
  let attempt = attempts.get(info);
  if (!attempt) {
    attempt = {
      info,
      hits: [],
      pages: [],
      errors: new Set(),
      file: resolve(directory, `${info.project.name}-${info.workerIndex}.jsonl`),
    };
    attempts.set(info, attempt);
  }
  return attempt;
}

async function attribute(hit, headers) {
  const { authorization, cookie } = await headers;
  if (authorization !== undefined) {
    hit.row.credential = /^Bearer(?:\s|$)/i.test(authorization) ? 'bearer' : 'authorization';
    return;
  }

  const token = sessionCookie.exec(cookie ?? '')?.[1];
  hit.row.credential = token ? 'session' : 'none';
  if (!token) {
    hit.row.actor = 'anonymous';
    return;
  }
  try {
    hit.row.actor = await sessionActor(token);
  } catch {
    hit.attempt.errors.add('session actor lookup');
  }
}

function start(url, method, headers) {
  const attempt = currentAttempt();
  if (!attempt) return null;
  const { info } = attempt;
  const target = new URL(url);
  const origin = new URL(info.project.use.baseURL).origin;
  const local = target.origin === origin;
  const row = {
    route_pattern: local ? routes.find(route => route.match.test(target.pathname))?.pattern ?? null : null,
    method,
    status: null,
    actor: null,
    credential: null,
    form_action: null,
    spec: relative(testDirectory, info.file),
    test: info.titlePath.slice(1).join(' › '),
    project: info.project.name,
    worker: info.workerIndex,
    retry: info.retry,
  };
  const hit = { attempt, row, local };
  if (local) {
    hit.actor = attribute(hit, headers).catch(() => {
      // Chromium may drop header data for requests that never received a response.
      hit.missingHeaders = true;
    });
  }
  attempt.hits.push(hit);
  return hit;
}

export async function recordedPrincipals(info, route, method) {
  const hits = attempts.get(info)?.hits.filter(({row}) => row.route_pattern === route && row.method === method) ?? [];
  await Promise.all(hits.map(hit => hit.actor));
  return hits.map(({row: {actor, credential, status}}) => ({actor, credential, status}));
}

export async function flushRecorder(info) {
  const attempt = attempts.get(info);
  if (!attempt) return;
  await Promise.all(attempt.pages);
  await Promise.all(attempt.hits.flatMap(hit => [hit.actor, hit.form]));
  for (const hit of attempt.hits) {
    if (hit.row.status !== null && hit.missingHeaders) attempt.errors.add('request header capture');
  }
  const lines = attempt.hits.map(hit => hit.row);
  if (attempt.errors.size) lines.push({ recorder_errors: [...attempt.errors].sort() });
  mkdirSync(directory, { recursive: true });
  const capture = info.config.metadata.ledger;
  appendFileSync(attempt.file, lines.map(row => JSON.stringify({capture, ...row}) + '\n').join(''));
  attempts.delete(info);
  if (attempt.errors.size) throw new Error(`Ledger recorder failed: ${[...attempt.errors].sort().join(', ')}`);
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
      const { url, method, headers } = event;
      const credentials = Object.fromEntries(Object.entries(headers).map(([key, value]) => [key.toLowerCase(), value]));
      apiRequests.set(event, start(url, method, credentials));
    });
    implementation.on('requestfinished', event => {
      const hit = apiRequests.get(event.requestEvent);
      if (hit) hit.row.status = event.statusCode;
    });
  };
  const watchForms = async (context, page) => {
    const cdp = await context.newCDPSession(page);
    const requested = new Map();
    const submissions = new Map();

    cdp.on('Page.frameRequestedNavigation', ({ frameId, reason, url }) => {
      if (reason === 'formSubmissionGet' || reason === 'formSubmissionPost') {
        requested.set(frameId, { url, method: reason === 'formSubmissionGet' ? 'GET' : 'POST' });
      } else {
        requested.delete(frameId);
      }
    });
    cdp.on('Page.frameStartedNavigating', ({ frameId, loaderId, url, navigationType }) => {
      const submission = requested.get(frameId);
      requested.delete(frameId);
      if (navigationType === 'differentDocument' && submission?.url === url) {
        submissions.set(loaderId, submission);
      }
    });
    await cdp.send('Page.enable');
    return submissions;
  };
  playwright._instrumentation.addListener({
    runAfterCreateBrowserContext(context) {
      watchApi(context.request);
      const pageForms = new WeakMap();
      context.on('page', page => {
        const attempt = currentAttempt();
        const forms = watchForms(context, page).catch(() => {
          if (!page.isClosed()) attempt?.errors.add('browser form capture');
          return null;
        });
        pageForms.set(page, forms);
        attempt?.pages.push(forms);
      });
      context.on('request', request => {
        const headers = request.allHeaders().catch(() => {
          // Attach a rejection handler immediately, including on external requests.
          return null;
        });
        browserRequests.set(request, start(request.url(), request.method(), headers));
      });
      context.on('response', response => {
        const request = response.request();
        const hit = browserRequests.get(request);
        if (!hit) return;
        hit.row.status = response.status();
        if (!hit.local || !request.isNavigationRequest() || request.redirectedFrom()) return;
        hit.form = (async () => {
          // Chromium document IDs are CDP loader IDs. Only the original
          // navigation shares both the submitted destination and method.
          const document = request._connection.toImpl(request)._documentId;
          const submissions = await pageForms.get(request.frame().page());
          if (!submissions || !document) throw new Error('Missing Chromium form capture');
          const submission = submissions.get(document);
          submissions.delete(document);
          if (submission?.method === request.method() && submission.url === request.url()) {
            const headers = request.headers();
            hit.row.form_action = formAction(new URL(request.url()), request.method(),
              headers['content-type'] ?? '', request.postDataBuffer(), headers.referer);
          }
        })().catch(() => {
          hit.attempt.errors.add('browser form capture');
        });
      });
    },
    runAfterCreateRequestContext: watchApi,
  });
}
