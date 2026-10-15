FROM docker:29-cli@sha256:1a4c7cb63513f349bdad01fcc6e0f3f2f67d37b9da86f14dc0d4a0942eecda00
RUN apk add --no-cache bash python3 github-cli jq ca-certificates
WORKDIR /src
ENTRYPOINT ["bash", "/src/deploy/oracle/bootstrap-native.sh"]
