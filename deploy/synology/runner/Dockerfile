FROM docker:29.6.2-cli@sha256:feb2d49bd65f274b3e4b4620beabe2f4691e5287e496da9fbc9830ed5f780676 AS docker-cli

FROM ubuntu:24.04@sha256:019e8eb29a85e74d64925745884f2ec79aa27e3feab36353d24656f4d6b89467

ENV RUNNER_VERSION=2.336.0 \
    RUNNER_SHA256=04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d \
    DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates coreutils curl git gzip jq sudo tar \
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
