# Stock Caddy plus the caddy-l4 listener wrapper, so native gameplay can share
# :443 by SNI and ALPN (see Caddyfile). Build on the arm64 host:
#   docker build -f deploy/oracle/edge.Dockerfile -t oteryn-edge:2.11.7-l4-0.1.3 deploy/oracle
# Rollback: set edge.compose.yml back to the stock image named on the final
# stage below and restore the Caddyfile without `listener_wrappers`.
FROM caddy:2.11.7-builder-alpine@sha256:95a5e7e2692b5eb9545aadc5b90a39f6f509bee59c9508a493f83ee902eaef63 AS build
RUN xcaddy build v2.11.7 --with github.com/mholt/caddy-l4@v0.1.3

FROM caddy:2-alpine@sha256:d8542f48d34a9cf4e4c11a478865229840e87e4c96ea3f439101f31a5d35f75f
COPY --from=build /usr/bin/caddy /usr/bin/caddy
RUN caddy list-modules | grep -qx layer4.handlers.proxy
