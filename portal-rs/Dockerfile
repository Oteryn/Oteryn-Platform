# Container image for the `oteryn-portal` binary (roles: serve, worker,
# migrate, gateway). Build from this directory:
#
#   docker build -t oteryn-portal portal-rs
#
# Run one role per container, for example `docker run ... oteryn-portal worker`.
# Configuration comes only from `OTERYN_*` / `PORTAL_*` environment variables
# supplied at run time (see README.md); no secret, key or `.env` file is copied
# into the image. Certificates and keys are mounted read-only and named by
# their `*_FILE` variables.
#
# Base images are pinned by digest; bump the tag and digest together.

FROM docker.io/library/rust:1.97.0-slim-trixie@sha256:686a437ead83701e8f871e66e838c3ec55f46b5fc235b025756396ac823bdc51 AS build
# Matches rust-toolchain.toml without installing its lint components.
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

# Distroless C runtime: glibc and CA certificates, no shell, uid 65532.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2
WORKDIR /app
COPY --from=build /out/oteryn-portal /usr/local/bin/oteryn-portal
COPY --chown=0:0 static /app/static
# Non-secret defaults only; override at run time.
ENV OTERYN_BIND=0.0.0.0:8080 \
    OTERYN_STATIC_DIR=/app/static
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/oteryn-portal"]
CMD ["serve"]
