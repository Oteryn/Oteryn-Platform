# syntax=docker/dockerfile:1.7

ARG PHP_IMAGE=php:8.5-cli-bookworm
ARG NODE_IMAGE=node:22-bookworm-slim
ARG COMPOSER_IMAGE=composer:2

FROM ${NODE_IMAGE} AS node_runtime
FROM ${COMPOSER_IMAGE} AS composer_runtime
FROM ${PHP_IMAGE}

ENV DEBIAN_FRONTEND=noninteractive \
    COMPOSER_ALLOW_SUPERUSER=1 \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    PATH=/opt/oteryn-playwright/node_modules/.bin:/usr/local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

COPY --from=node_runtime /usr/local/ /usr/local/
COPY --from=composer_runtime /usr/bin/composer /usr/local/bin/composer

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        $PHPIZE_DEPS \
        bash \
        ca-certificates \
        coreutils \
        curl \
        default-mysql-client \
        git \
        imagemagick \
        libfreetype6-dev \
        libicu-dev \
        libjpeg62-turbo-dev \
        libonig-dev \
        libpng-dev \
        libwebp-dev \
        libzip-dev \
        procps \
        python3 \
        redis-tools \
        socat \
        tar \
        unzip \
    && docker-php-ext-configure gd --with-freetype --with-jpeg --with-webp \
    && docker-php-ext-install -j"$(nproc)" gd intl pcntl pdo_mysql zip \
    && pecl install redis \
    && docker-php-ext-enable redis \
    && php -r 'exit(PHP_VERSION_ID >= 80500 ? 0 : 1);' \
    && php -r 'exit(extension_loaded("dom") && extension_loaded("gd") && extension_loaded("intl") && extension_loaded("mbstring") && extension_loaded("pcntl") && extension_loaded("pdo_mysql") && extension_loaded("redis") && extension_loaded("xml") && extension_loaded("xmlwriter") && extension_loaded("zip") ? 0 : 1);' \
    && node --version \
    && npm --version \
    && composer --version \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/oteryn-playwright
COPY scripts/acceptance/package.json ./package.json

RUN mkdir -p "$PLAYWRIGHT_BROWSERS_PATH" \
    && npm install --no-audit --no-fund --package-lock=false \
    && node -e 'const installed = require("./node_modules/@playwright/test/package.json").version; const expected = require("./package.json").devDependencies["@playwright/test"]; if (installed !== expected) { console.error(`Playwright version mismatch: installed ${installed}, expected ${expected}`); process.exit(1); }' \
    && npx playwright install --with-deps chromium firefox webkit \
    && npx playwright --version \
    && chmod -R a+rX /opt/oteryn-playwright "$PLAYWRIGHT_BROWSERS_PATH" \
    && npm cache clean --force \
    && rm -rf /var/lib/apt/lists/*

COPY scripts/acceptance/run-playwright-ci.sh /usr/local/bin/oteryn-playwright-ci
RUN chmod 0755 /usr/local/bin/oteryn-playwright-ci

WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/oteryn-playwright-ci"]
CMD ["--verify-only"]
