import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import { stagingHost, stagingProxy } from './transport.mjs';

const authority = `${stagingHost}:443`;
const upstream = stagingProxy && new URL(stagingProxy.server);

function tunnel(client, head) {
  function connected(remote, buffered = Buffer.alloc(0)) {
    if (client.destroyed) {
      remote.destroy();
      return;
    }
    remote.on('error', () => client.destroy());
    client.on('close', () => remote.destroy());
    client.write('HTTP/1.1 200 Connection Established\r\n\r\n');
    if (buffered.length) client.write(buffered);
    if (head.length) remote.write(head);
    client.pipe(remote).pipe(client);
  }

  if (!upstream) {
    const remote = net.connect(443, stagingHost, () => connected(remote));
    remote.on('error', () => client.destroy());
    client.on('close', () => remote.destroy());
    return;
  }

  const agent = upstream.protocol === 'https:' ? https : http;
  const connect = agent.request(upstream, {
    method: 'CONNECT',
    path: authority,
    headers: { host: authority },
  });
  connect.once('connect', (response, socket, buffered) => {
    if (response.statusCode === 200) connected(socket, buffered);
    else {
      socket.destroy();
      client.end('HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n');
    }
  });
  connect.on('error', () => client.destroy());
  connect.setTimeout(12_000, () => connect.destroy());
  client.on('close', () => connect.destroy());
  connect.end();
}

// Playwright also applies route header overrides to browser redirects. Limit
// Chromium's CONNECT destination while leaving the staging TLS traffic intact.
export async function startStagingBrowserProxy() {
  const server = http.createServer((_request, response) => response.writeHead(403).end());
  const clients = new Set();
  server.on('connection', socket => {
    clients.add(socket);
    socket.on('error', () => socket.destroy());
    socket.once('close', () => clients.delete(socket));
  });
  server.on('connect', (request, client, head) => {
    if (request.url !== authority) {
      client.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n');
      return;
    }
    tunnel(client, head);
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  return {
    // Apply the explicit proxy to loopback URLs too.
    options: { server: `http://127.0.0.1:${server.address().port}`, bypass: '<-loopback>' },
    close: async () => {
      for (const client of clients) client.destroy();
      await new Promise(resolve => server.close(resolve));
    },
  };
}
