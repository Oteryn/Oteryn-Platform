FROM golang:1.26.7-alpine@sha256:28d89ee9cc0ff9fec75c82ca201e6bf7fdf9a679d4b7b24dfa04f2bb766bb468 AS build

WORKDIR /src/services/game-gateway
COPY services/game-gateway/go.mod ./
COPY services/game-gateway/cmd/ ./cmd/
COPY services/game-gateway/internal/ ./internal/
RUN CGO_ENABLED=0 GOOS=linux go build \
    -trimpath \
    -ldflags="-s -w" \
    -o /out/game-gateway \
    ./cmd/game-gateway

FROM alpine:3.24.2@sha256:31b6477333eb8257db9e5d7c3a7264fd0467928756f0bbcc27d35bea5d28cdbd

RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 10001 oteryn \
    && adduser -S -D -H -u 10001 -G oteryn oteryn

COPY --from=build /out/game-gateway /usr/local/bin/game-gateway

USER oteryn

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/game-gateway"]
