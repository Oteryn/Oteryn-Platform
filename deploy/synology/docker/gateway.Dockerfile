FROM golang:1.27.2-alpine@sha256:f92b6ef800e499660581efdabdf25d9d817a9d124eaf900924f0504e7e27e12d AS build

WORKDIR /src/services/game-gateway
COPY services/game-gateway/go.mod ./
COPY services/game-gateway/cmd/ ./cmd/
COPY services/game-gateway/internal/ ./internal/
RUN CGO_ENABLED=0 GOOS=linux go build \
    -trimpath \
    -ldflags="-s -w" \
    -o /out/game-gateway \
    ./cmd/game-gateway

FROM alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6

RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 10001 oteryn \
    && adduser -S -D -H -u 10001 -G oteryn oteryn

COPY --from=build /out/game-gateway /usr/local/bin/game-gateway

USER oteryn

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/game-gateway"]
