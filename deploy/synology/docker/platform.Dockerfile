FROM composer:2@sha256:af98f42dfff7c68ba8d53c2164fd9fde1087b7d449514baa38c418b1f6bc4bac AS composer-bin

# The FPM variant of the same PHP 8.5.11 Alpine 3.24 release. It still ships the PHP CLI that runs
# `php artisan serve` (the default command) and adds php-fpm for the optional mTLS FastCGI listener
# behind the Synology internal-mtls terminator (README: Platform internal mTLS listener).
FROM php:8.5-fpm-alpine@sha256:fa01fb1645cd0fc566a5f146b099adace33b906571f972f71f2182a7c12d1cd7

RUN apk add --no-cache \
        freetype \
        icu-libs \
        libjpeg-turbo \
        libpng \
        libpq \
        libwebp \
        libzip \
        oniguruma \
        unzip \
    && apk add --no-cache --virtual .build-deps \
        $PHPIZE_DEPS \
        freetype-dev \
        icu-dev \
        libjpeg-turbo-dev \
        libpng-dev \
        libwebp-dev \
        libzip-dev \
        oniguruma-dev \
        postgresql-dev \
    && docker-php-ext-configure gd --with-freetype --with-jpeg --with-webp \
    && docker-php-ext-install -j"$(nproc)" gd intl mbstring pcntl pdo_mysql pdo_pgsql zip \
    && apk del .build-deps \
    && php -r 'exit(extension_loaded("gd") && (imagetypes() & IMG_JPG) && (imagetypes() & IMG_PNG) && (imagetypes() & IMG_WEBP) ? 0 : 1);' \
    && php -r 'exit(extension_loaded("pdo_mysql") && extension_loaded("pdo_pgsql") ? 0 : 1);'

COPY --from=composer-bin /usr/bin/composer /usr/local/bin/composer
COPY deploy/synology/docker/platform-media.ini /usr/local/etc/php/conf.d/zz-oteryn-media.ini
COPY deploy/synology/docker/platform-entrypoint.sh /usr/local/bin/oteryn-platform-entrypoint
RUN chmod 0755 /usr/local/bin/oteryn-platform-entrypoint \
    && /usr/local/bin/oteryn-platform-entrypoint --self-test

# One FastCGI pool for the Synology internal-mtls terminator, started by the entrypoint only when
# OTERYN_PLATFORM_FASTCGI_ENABLED=true. It listens on a unix socket in a volume shared only with the
# terminator (never on TCP), so no other container can speak FastCGI and forge the SSL_CLIENT_*
# parameters the internal mTLS middleware trusts. The image default pools (TCP 9000) are removed.
# clear_env=no keeps the entrypoint-exported runtime environment, exactly as `artisan serve` sees it.
# Static workers: the Game node polls character commands about once a second through this pool, and a
# native-evidence read queued behind those polls missed the node's 3 s exchange deadline with 4 ondemand workers.
RUN rm -f /usr/local/etc/php-fpm.d/*.conf \
    && printf '%s\n' \
        '[global]' \
        'pid = /tmp/oteryn-php-fpm.pid' \
        'error_log = /proc/self/fd/2' \
        'log_limit = 8192' \
        'daemonize = no' \
        '' \
        '[internal-mtls]' \
        'listen = /run/oteryn-fastcgi/php-fpm.sock' \
        'listen.mode = 0666' \
        'pm = static' \
        'pm.max_children = 12' \
        'pm.max_requests = 500' \
        'request_terminate_timeout = 30s' \
        'clear_env = no' \
        'catch_workers_output = yes' \
        'decorate_workers_output = no' \
        'security.limit_extensions = .php' \
        > /usr/local/etc/php-fpm.d/oteryn-internal-mtls.conf \
    && install -d -o www-data -g www-data -m 0750 /run/oteryn-fastcgi

WORKDIR /var/www/html

COPY composer.json composer.lock ./
RUN composer install \
    --no-dev \
    --no-interaction \
    --no-progress \
    --no-scripts \
    --optimize-autoloader \
    --prefer-dist

# Keep the application layer cache key bound to real Platform runtime inputs.
# Repository governance, tests, CI and unrelated deployment assets must not
# invalidate a production Platform image layer.
COPY artisan ./artisan
COPY app/ ./app/
COPY bootstrap/ ./bootstrap/
COPY config/ ./config/
COPY database/ ./database/
COPY public/ ./public/
COPY resources/ ./resources/
COPY routes/ ./routes/
COPY lang/ ./lang/
COPY storage/ ./storage/
COPY deploy/synology/release-contract.env ./deploy/synology/release-contract.env

# The reviewed Wiki launch-content commands validate these exact repository
# provenance files at runtime. Copy only that bounded set rather than all docs.
COPY docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json ./docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
COPY docs/architecture/adr/0004-authoritative-platform-account-ownership.md ./docs/architecture/adr/0004-authoritative-platform-account-ownership.md
COPY docs/architecture/adr/0005-character-creation-product-policy.md ./docs/architecture/adr/0005-character-creation-product-policy.md
COPY docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md ./docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
COPY docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md ./docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md
COPY docs/agents/PROJECT_STATE.md ./docs/agents/PROJECT_STATE.md
COPY docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md ./docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
COPY docs/architecture/SECURITY_ARCHITECTURE.md ./docs/architecture/SECURITY_ARCHITECTURE.md
COPY docs/architecture/adr/0013-wiki-administration.md ./docs/architecture/adr/0013-wiki-administration.md

RUN composer dump-autoload --no-dev --no-interaction --optimize \
    && mkdir -p \
        storage/framework/cache \
        storage/framework/sessions \
        storage/framework/views \
        storage/logs \
    && chown -R www-data:www-data storage bootstrap/cache

USER www-data

RUN php-fpm --test

EXPOSE 8000

# The FPM base image stops with SIGQUIT (graceful FPM shutdown); the default command is still
# `artisan serve`, so keep the CLI base image's SIGTERM.
STOPSIGNAL SIGTERM

ENTRYPOINT ["/usr/local/bin/oteryn-platform-entrypoint"]
CMD ["php", "artisan", "serve", "--host=0.0.0.0", "--port=8000"]
