# Build with an immutable Oteryn-Game source archive as the context.
FROM rust:1.94.0-bookworm@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --locked --release --jobs 2 -p oteryn-game-server --bins

FROM build AS world-bundle-build
# Archive inputs have no Git metadata. Index only the compiler's documented
# inputs; pin-check verifies their blob identities against the committed pin.
# No synthetic commit or history is created.
RUN git init \
    && git add --force -- content/world content/houses content/creatures/definitions \
        content/items/definitions content/npcs/definitions content/dialogues/definitions \
        content/services tools/world-bundle-compiler crates/world-bundle \
        Cargo.toml Cargo.lock rust-toolchain.toml .cargo \
    && cargo build --locked --release --jobs 2 -p oteryn-world-bundle-compiler \
    && target/release/oteryn-world-bundle-compiler pin-check . /world-bundle

FROM scratch AS world-bundle
COPY --from=world-bundle-build /world-bundle/ /

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/oteryn-game-server /usr/local/bin/oteryn-game-server
COPY --from=build /src/target/release/oteryn-game-ops /usr/local/bin/oteryn-game-ops
COPY --from=build /src/target/release/oteryn-game-migrate /usr/local/bin/oteryn-game-migrate
USER 1001:1001
ENTRYPOINT ["/usr/local/bin/oteryn-game-server"]
CMD ["--help"]
