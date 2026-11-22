import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import { SMTPServer } from 'smtp-server';

function listen(server, port) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => {
      server.removeListener('error', reject);
      resolve();
    });
  });
}

export async function startOutboxBrowser({ upstreamPort, certificate, caFile, env }) {
  const tls = {
    cert: fs.readFileSync(certificate.cert),
    key: fs.readFileSync(certificate.key),
  };
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
  await listen(sink, 587);
  const proxy = https.createServer(tls, (request, response) => {
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
  try {
    await listen(proxy, 0);
  } catch (error) {
    sink.close();
    throw error;
  }
  Object.assign(env, {
    SSL_CERT_FILE: caFile,
    OTERYN_PUBLIC_ORIGIN: `https://localhost:${proxy.address().port}`,
    PORTAL_E2E_ORIGIN: `https://localhost:${proxy.address().port}`,
  });
  return () => {
    proxy.close();
    sink.close();
  };
}
