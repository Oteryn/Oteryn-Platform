FROM golang:1.24-bookworm AS build

WORKDIR /src
COPY go.mod ./
COPY cmd ./cmd
COPY internal ./internal

RUN CGO_ENABLED=0 GOOS=linux go build -trimpath -ldflags="-s -w" -o /out/oteryn-game-gateway ./cmd/game-gateway

FROM scratch

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /out/oteryn-game-gateway /oteryn-game-gateway

USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/oteryn-game-gateway"]
