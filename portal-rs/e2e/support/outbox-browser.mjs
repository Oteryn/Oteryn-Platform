import crypto from 'node:crypto';
import fs from 'node:fs';
import crypto from 'node:crypto';
import http from 'node:http';
import https from 'node:https';
import { SMTPServer } from 'smtp-server';
import { mailSinkOptions } from './mail-sink.mjs';

function listen(server, port) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => {
      server.removeListener('error', reject);
      // SMTP forwards connection errors after startup, including failed TLS handshakes.
      server.on('error', () => console.error('[harness] loopback listener error'));
      resolve();
    });
  });
}

function credentials(certificate) {
  return {
    cert: fs.readFileSync(certificate.cert),
    key: fs.readFileSync(certificate.key),
  };
}

function browserProxy(tls, upstreamPort) {
  return https.createServer(tls, (request, response) => {
    const upstream = http.request({
      hostname: '127.0.0.1', port: upstreamPort, path: request.url,
      method: request.method, headers: request.headers,
    }, result => {
      response.writeHead(result.statusCode, result.headers);
      result.pipe(response);
    });
    upstream.on('error', () => {
      response.writeHead(502);
      response.end();
    });
    request.pipe(upstream);
  });
}

function configureBrowserOrigin(env, port) {
  const origin = new URL(`https://localhost:${port}`).origin;
  env.PORTAL_E2E_ORIGIN = origin;
  if (env.PORTAL_E2E_BROWSER_HTTPS_PORT) {
    env.OTERYN_NOTIFY_UNSUBSCRIBE_KEY = crypto.randomBytes(32).toString('hex');
    env.OTERYN_NOTIFY_UNSUBSCRIBE_URL = `${origin}/en/unsubscribe`;
  }
  return origin;
}

export async function startOutboxBrowser({ upstreamPort, certificate, caFile, env }) {
  const tls = credentials(certificate);
  const sink = new SMTPServer({
    ...tls,
    onAuth(auth, _session, callback) {
      if (auth.username !== env.OTERYN_SMTP_USERNAME || auth.password !== env.OTERYN_SMTP_PASSWORD) {
        callback(new Error('Local SMTP authentication refused'));
        return;
      }
      callback(null, { user: auth.username });
    },
    onData(stream, _session, callback) {
      stream.on('error', callback);
      stream.on('end', () => callback());
      stream.resume();
    },
  });
  // Identity notifications use the same authenticated STARTTLS adapter as local.mjs.
  await listen(sink, 0);
  const proxy = browserProxy(tls, upstreamPort);
  try {
    await listen(proxy, Number(env.PORTAL_E2E_BROWSER_HTTPS_PORT || 0));
  } catch (error) {
    sink.close();
    throw error;
  }
  Object.assign(env, {
    SSL_CERT_FILE: caFile,
    OTERYN_SMTP_PORT: String(sink.server.address().port),
    OTERYN_SMTP_TLS_MODE: 'starttls',
    OTERYN_PUBLIC_ORIGIN: configureBrowserOrigin(env, proxy.address().port),
  });
  if (env.PORTAL_E2E_GM_ENFORCE === '1') {
    Object.assign(env, {
      OTERYN_ADMIN_CONSOLE: 'true',
      OTERYN_ADMIN_PANEL: 'true',
      OTERYN_ADMIN_HOST: new URL(env.OTERYN_PUBLIC_ORIGIN).host,
      OTERYN_ACCOUNT_MFA: 'true',
      OTERYN_NOTIFY_UNSUBSCRIBE_KEY: crypto.randomBytes(32).toString('hex'),
      OTERYN_NOTIFY_UNSUBSCRIBE_URL: new URL('/en/unsubscribe', env.OTERYN_PUBLIC_ORIGIN).href,
      PORTAL_GAME_ENFORCEMENT_ENABLED: 'true',
      PORTAL_GAME_ENFORCEMENT_PROFILES: '["MUTE","JAIL"]',
      // The accepted contract keeps results off on a restore-enabled stack.
      PORTAL_GAME_RESTORE_NOTICE_ENABLED: 'false',
    });
  }
  return () => {
    proxy.close();
    sink.close();
  };
}

export async function startStaffBrowser({ upstreamPort, certificate, caFile, env, browserPort }) {
  const tls = credentials(certificate);
  const username = 'local';
  const password = crypto.randomBytes(24).toString('hex');
  configureBrowserOrigin(env, browserPort);
  Object.assign(env, {
    OTERYN_SMTP_HOST: 'localhost',
    OTERYN_SMTP_USERNAME: username,
    OTERYN_SMTP_PASSWORD: password,
    OTERYN_SMTP_FROM: 'portal@local.test',
    OTERYN_MAIL_FROM: 'portal@local.test',
    SSL_CERT_FILE: caFile,
  });
  const listeners = [];
  const messages = [];
  const close = () => {
    for (const listener of listeners) listener.close();
  };
  try {
    for (const port of [465, 587]) {
      const sink = new SMTPServer(mailSinkOptions({ tls, secure: port === 465, username, password, messages }));
      await listen(sink, port);
      listeners.push(sink);
    }
    const proxy = browserProxy(tls, upstreamPort);
    await listen(proxy, browserPort);
    listeners.push(proxy);
  } catch (error) {
    close();
    throw error;
  }
  return close;
}
