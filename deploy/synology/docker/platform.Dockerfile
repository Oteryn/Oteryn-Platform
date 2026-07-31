FROM php:8.5-cli-alpine

RUN apk add --no-cache \
        freetype \
        icu-libs \
        libjpeg-turbo \
        libpng \
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
    && docker-php-ext-configure gd --with-freetype --with-jpeg --with-webp \
    && docker-php-ext-install -j"$(nproc)" gd intl mbstring pcntl pdo_mysql zip \
    && pecl install redis \
    && docker-php-ext-enable redis \
    && apk del .build-deps \
    && php -r 'exit(extension_loaded("gd") && (imagetypes() & IMG_JPG) && (imagetypes() & IMG_PNG) && (imagetypes() & IMG_WEBP) ? 0 : 1);'

COPY --from=composer:2 /usr/bin/composer /usr/local/bin/composer
COPY deploy/synology/docker/platform-media.ini /usr/local/etc/php/conf.d/zz-oteryn-media.ini
COPY deploy/synology/docker/platform-entrypoint.sh /usr/local/bin/oteryn-platform-entrypoint
RUN chmod 0755 /usr/local/bin/oteryn-platform-entrypoint \
    && /usr/local/bin/oteryn-platform-entrypoint --self-test

WORKDIR /var/www/html

COPY composer.json composer.lock ./
RUN composer install \
    --no-dev \
    --no-interaction \
    --no-progress \
    --no-scripts \
    --optimize-autoloader \
    --prefer-dist

COPY . .
RUN composer dump-autoload --no-dev --no-interaction --optimize \
    && mkdir -p \
        storage/framework/cache \
        storage/framework/sessions \
        storage/framework/views \
        storage/logs \
    && chown -R www-data:www-data storage bootstrap/cache

USER www-data

EXPOSE 8000

ENTRYPOINT ["/usr/local/bin/oteryn-platform-entrypoint"]
CMD ["php", "artisan", "serve", "--host=0.0.0.0", "--port=8000"]
