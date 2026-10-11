FROM golang:1.27.0-alpine@sha256:4c9fe60190a2a3350ddc51de80d0224b8a6698d12bdfc999fee45ea9d6c46dbc AS build

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
