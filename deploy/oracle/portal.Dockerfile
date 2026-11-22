# Image for the live portal on oteryn-vps (/opt/oteryn-portal), built by
# actions-deploy.py from the qualified source archive with `portal-rs` as the
# context:
#
#   docker build -f deploy/oracle/portal.Dockerfile -t oteryn-portal:<short8>-arm64 portal-rs
#
# It matches the image the live composition already runs: uid 10001, binary
# at /app/oteryn-portal, /app/static and /app/migrations, ports 8080/8091.
# portal-rs/Dockerfile differs (distroless, uid 65532), and the live secrets
# and volumes are owned for uid 10001. No secret or `.env` enters the image.
#
# The build stage is the one in portal-rs/Dockerfile; bump both together.

FROM docker.io/library/rust:1.97.0-slim-trixie@sha256:686a437ead83701e8f871e66e838c3ec55f46b5fc235b025756396ac823bdc51 AS build
ENV RUSTUP_TOOLCHAIN=1.97.0 \
    CARGO_TERM_COLOR=never
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY bin ./bin
COPY crates ./crates
COPY migrations ./migrations
COPY templates ./templates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked --release -p oteryn-portal \
    && install -D -m 0755 target/release/oteryn-portal /out/oteryn-portal

FROM ubuntu:26.04
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 portal
WORKDIR /app
COPY --from=build /out/oteryn-portal /app/oteryn-portal
COPY --chown=0:0 static /app/static
COPY --chown=0:0 migrations /app/migrations
USER portal
EXPOSE 8080 8091
# Keep the legal gate in `migrate` when an older installed helper runs this image.
ENV OTERYN_ORACLE_DEPLOY_IMAGE=1
ENTRYPOINT ["/app/oteryn-portal"]
