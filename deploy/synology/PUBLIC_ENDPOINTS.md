# Synology public endpoint mapping

This file complements `deploy/synology/README.md` with the owner-designated Cloudflare Tunnel hostnames for the current Oteryn deployment.

## Cloudflare Tunnel routes

```text
https://oteryn.molehill.cloud
  -> Oteryn Platform web server
  -> http://127.0.0.1:8000

https://login.oteryn.molehill.cloud
  -> Oteryn Game Gateway / native client login API
  -> http://127.0.0.1:8080
```

Do not swap these origins:

- port `8000` is the Platform website;
- port `8080` is Game Gateway/login;
- port `7171` is only the Canary legacy-login rollback path;
- port `7172` is the Canary game-protocol TCP endpoint and is not an HTTP Cloudflare Tunnel origin;
- port `3031` belongs to a different project.

The loopback origins are intentional. The current deployment contract keeps Platform and Gateway bound to Synology host loopback rather than `192.168.1.2`.

The durable cross-component contract is `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md`.
