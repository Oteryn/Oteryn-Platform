FROM docker:29.8.1-cli@sha256:6602978e2be3c20e530e33773b8cadcef5fe998a71a534ee24516f1176973cdf AS docker-cli

FROM ubuntu:24.04@sha256:019e8eb29a85e74d64925745884f2ec79aa27e3feab36353d24656f4d6b89467

ENV RUNNER_VERSION=2.337.0 \
    RUNNER_SHA256=70920811a4f8ad4328818682bca5c6469c1c942fab52448868071d0063816613 \
    DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates coreutils curl git gzip jq python3 sudo tar \
    && python3 --version \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1001 --shell /bin/bash runner \
    && mkdir -p /opt/actions-runner-dist /runner /work

RUN curl --fail --show-error --silent --location \
        "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" \
        --output /tmp/actions-runner.tar.gz \
    && printf '%s  %s\n' "$RUNNER_SHA256" /tmp/actions-runner.tar.gz | sha256sum --check --strict \
    && tar -xzf /tmp/actions-runner.tar.gz -C /opt/actions-runner-dist \
    && rm -f /tmp/actions-runner.tar.gz \
    && /opt/actions-runner-dist/bin/installdependencies.sh \
    && for node_root in /opt/actions-runner-dist/externals/node20 /opt/actions-runner-dist/externals/node24; do \
        test -x "$node_root/bin/node"; \
        rm -rf "$node_root/lib/node_modules"; \
        rm -f "$node_root/bin/npm" "$node_root/bin/npx" "$node_root/bin/corepack"; \
    done \
    && /opt/actions-runner-dist/externals/node24/bin/node --version \
    && test ! -e /opt/actions-runner-dist/externals/node24/bin/npm \
    && test -x /opt/actions-runner-dist/externals/node24/bin/node \
    && test "$(/opt/actions-runner-dist/bin/Runner.Listener --version 2>/dev/null | tail -n 1)" = "$RUNNER_VERSION" \
    && chown -R runner:runner /opt/actions-runner-dist /runner /work

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker-cli /usr/local/libexec/docker/cli-plugins/docker-compose /usr/local/libexec/docker/cli-plugins/docker-compose

COPY --chown=runner:runner deploy/synology/runner/entrypoint.sh /usr/local/bin/oteryn-runner-entrypoint
COPY deploy/synology/runner/test-entrypoint.sh /usr/local/bin/oteryn-runner-entrypoint-self-test
RUN chmod 0755 /usr/local/bin/oteryn-runner-entrypoint /usr/local/bin/oteryn-runner-entrypoint-self-test \
    && /usr/local/bin/oteryn-runner-entrypoint-self-test \
    && rm -f /usr/local/bin/oteryn-runner-entrypoint-self-test

USER runner
WORKDIR /runner

ENTRYPOINT ["/usr/local/bin/oteryn-runner-entrypoint"]
