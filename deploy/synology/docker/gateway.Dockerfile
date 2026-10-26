FROM golang:1.27.3-alpine@sha256:b8c9b82affbbb82a020bbcf998dad2ba9516944ac7e883524642a481a9505e9c AS build

WORKDIR /src/services/game-gateway
COPY services/game-gateway/go.mod ./
COPY services/game-gateway/cmd/ ./cmd/
COPY services/game-gateway/internal/ ./internal/
RUN CGO_ENABLED=0 GOOS=linux go build \
    -trimpath \
    -ldflags="-s -w" \
    -o /out/game-gateway \
    ./cmd/game-gateway

FROM alpine:3.24.3@sha256:be2656061b536c1883c4034790e21d0a09c5aaaf51968b89368a3c0371ea2c9a

RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 10001 oteryn \
    && adduser -S -D -H -u 10001 -G oteryn oteryn

COPY --from=build /out/game-gateway /usr/local/bin/game-gateway

USER oteryn

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/game-gateway"]
