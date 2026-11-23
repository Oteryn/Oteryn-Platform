FROM docker.io/library/rust:1.97.0-slim-trixie@sha256:686a437ead83701e8f871e66e838c3ec55f46b5fc235b025756396ac823bdc51 AS sources
ARG CARGO_BUILD_JOBS=2
ENV RUSTUP_TOOLCHAIN=1.97.0 CARGO_TERM_COLOR=never CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY bin ./bin
COPY crates ./crates
COPY migrations ./migrations
COPY templates ./templates

FROM sources AS build
RUN --mount=type=cache,id=oteryn-staging-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    cargo fetch --locked
RUN --network=none \
    --mount=type=cache,id=oteryn-staging-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=oteryn-staging-target,target=/src/target,sharing=locked \
    cargo build --offline --locked --release -p oteryn-portal \
    && install -D -m 0755 target/release/oteryn-portal /out/oteryn-portal

FROM sources AS builder
COPY static ./static
COPY test-support ./test-support
COPY docs ./docs
COPY --from=contracts . /docs/contracts
COPY --from=oracle prepare-platform.sh /deploy/oracle/prepare-platform.sh
RUN --mount=type=cache,id=oteryn-staging-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    cargo fetch --locked \
    && cp -a /usr/local/cargo/registry /tmp/registry
RUN mv /tmp/registry /usr/local/cargo/registry \
    && install -d -o 10001 -g 10001 /target
ENV HOME=/tmp CARGO_TARGET_DIR=/target CARGO_NET_OFFLINE=true RUST_TEST_THREADS=1
USER 10001:10001
ENTRYPOINT ["cargo"]

FROM docker.io/library/ubuntu:26.04@sha256:d66b366dac61b575bee2aef07e9c26e8070096fbe84a3d4d3eb0da069b5d76db AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 portal
WORKDIR /app
COPY --from=build /out/oteryn-portal /app/oteryn-portal
COPY static /app/static
COPY migrations /app/migrations
USER 10001:10001
ENTRYPOINT ["/app/oteryn-portal"]

FROM mcr.microsoft.com/playwright:v1.58.2-noble@sha256:6446946a1d9fd62d9ae501312a2d76a43ee688542b21622056a372959b65d63d AS browser
RUN apt-get update \
    && apt-get install -y --no-install-recommends openssl python3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 browser
WORKDIR /workspace/e2e
COPY --from=e2e package.json package-lock.json ./
RUN PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --no-audit --no-fund
COPY --from=e2e . .
COPY --from=runtime /app /app
# The Playwright base can have an older libc than the Rust builder. Seed
# fixtures execute the same portal binary with its builder's loader and libs.
COPY --from=build /usr/lib/*-linux-gnu/ld-linux-*.so.* /usr/lib/*-linux-gnu/libc.so.6 /usr/lib/*-linux-gnu/libm.so.6 /usr/lib/*-linux-gnu/libgcc_s.so.1 /portal-lib/
RUN printf '%s\n' '#!/bin/sh' 'exec /portal-lib/ld-linux-*.so.* --library-path /portal-lib /app/oteryn-portal "$@"' > /usr/local/bin/staging-portal \
    && chmod 0755 /usr/local/bin/staging-portal
COPY crates /workspace/crates
COPY bin /workspace/bin
COPY templates /workspace/templates
COPY --from=contracts fixtures /docs/contracts/fixtures
RUN ln -s /app/static /workspace/static
USER 10001:10001
ENTRYPOINT ["node"]
