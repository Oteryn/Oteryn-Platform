FROM docker:29.9.2-cli@sha256:5e6baf67325a79f64ed3bed6863954d5e05d0f3d4e57cbdb14809346814bbfdb AS docker-cli

# Stay on Ubuntu 24.04: the NAS kernel is 4.4, and the 26.04 userland (GNU tar 1.35 on glibc 2.43) extracts
# through openat2(2), which that kernel lacks, so every action download fails with "Function not implemented".
FROM ubuntu:24.10@sha256:cdf755952ed117f6126ff4e65810bf93767d4c38f5c7185b50ec1f1078b464cc

ENV RUNNER_VERSION=2.337.0 \
    RUNNER_SHA256=70920811a4f8ad4328818682bca5c6469c1c942fab52448868071d0063816613 \
    DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates coreutils curl gh git gzip jq python3 sudo tar \
    && python3 --version \
    && gh --version \
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
    && printf '%s\n' "${RUNNER_VERSION}-pruned-1" > /opt/actions-runner-dist/.oteryn-dist-revision \
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
