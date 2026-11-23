import { once } from 'node:events';
import { createServer } from 'node:http';
import { createConnection } from 'node:net';

export function originProxySettings(url) {
  const endpoint = new URL(url);
  if (endpoint.protocol !== 'http:' || endpoint.hostname !== '127.0.0.1' || !endpoint.port
    || endpoint.href !== `${endpoint.origin}/`) {
    throw new Error('The crawl browser requires the local origin proxy');
  }
  // Chromium's default proxy bypass also covers loopback on other ports.
  return { server: endpoint.origin, bypass: '<-loopback>' };
}

export async function startOriginProxy(origin) {
  const { protocol, hostname, host, port } = new URL(origin);
  if (protocol !== 'https:' || hostname !== 'localhost' || !port) {
    throw new Error('The crawl browser requires a loopback HTTPS origin');
  }

  const sockets = new Set();
  function track(socket) {
    sockets.add(socket);
    socket.on('error', () => socket.destroy());
    socket.on('close', () => sockets.delete(socket));
  }

  // Check every new destination at the transport layer, including redirects
  // Playwright page.route never sees. Only the fixture receives a TLS tunnel.
  const proxy = createServer((_request, response) => {
    response.writeHead(403, { connection: 'close' }).end();
  });
  proxy.on('connection', track);
  proxy.on('connect', (request, socket, head) => {
    if (request.url !== host) {
      socket.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n');
      return;
    }

    const upstream = createConnection({ host: '127.0.0.1', port: Number(port) });
    track(upstream);
    upstream.on('close', () => socket.destroy());
    socket.on('close', () => upstream.destroy());
    upstream.once('connect', () => {
      socket.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      if (head.length) upstream.write(head);
      socket.pipe(upstream).pipe(socket);
    });
  });
  proxy.listen(0, '127.0.0.1');
  await once(proxy, 'listening');
  return {
    url: `http://127.0.0.1:${proxy.address().port}`,
    close() {
      proxy.close();
      for (const socket of sockets) socket.destroy();
    },
  };
}
