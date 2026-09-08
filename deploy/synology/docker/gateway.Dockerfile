FROM golang:1.24-alpine@sha256:8bee1901f1e530bfb4a7850aa7a479d17ae3a18beb6e09064ed54cfd245b7191 AS build

WORKDIR /src/services/game-gateway
COPY services/game-gateway/go.mod ./
COPY services/game-gateway/cmd/ ./cmd/
COPY services/game-gateway/internal/ ./internal/
RUN CGO_ENABLED=0 GOOS=linux go build \
    -trimpath \
    -ldflags="-s -w" \
    -o /out/game-gateway \
    ./cmd/game-gateway

FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce

RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 10001 oteryn \
    && adduser -S -D -H -u 10001 -G oteryn oteryn

COPY --from=build /out/game-gateway /usr/local/bin/game-gateway

USER oteryn

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/game-gateway"]
