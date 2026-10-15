# Build with an immutable Oteryn-Game source archive as the context.
FROM rust:1.94.0-bookworm@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --locked --release --jobs 2 -p oteryn-game-server --bins

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/oteryn-game-server /usr/local/bin/oteryn-game-server
COPY --from=build /src/target/release/oteryn-game-ops /usr/local/bin/oteryn-game-ops
COPY --from=build /src/target/release/oteryn-game-migrate /usr/local/bin/oteryn-game-migrate
USER 1001:1001
ENTRYPOINT ["/usr/local/bin/oteryn-game-server"]
CMD ["--help"]
