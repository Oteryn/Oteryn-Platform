ARG BASE_IMAGE
FROM ${BASE_IMAGE}

ARG RELEASE_SHA
ARG COMPONENT_SOURCE_SHA

LABEL org.opencontainers.image.revision="${RELEASE_SHA}" \
      io.oteryn.component.source-revision="${COMPONENT_SOURCE_SHA}" \
      io.oteryn.component.reused="true"
