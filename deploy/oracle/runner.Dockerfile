FROM ghcr.io/actions/actions-runner:2.338.0@sha256:4ffadc0002b2581327e06101fc8c06cd189232baf79fe561fac9caeb76f5e807
USER root
RUN apt-get update && apt-get install -y --no-install-recommends openssh-client python3 git curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
USER runner
WORKDIR /home/runner
ENTRYPOINT ["/bin/bash", "./run.sh"]
