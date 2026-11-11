<?php

return [
    'supported' => ['en', 'pl'],
    'default' => env('PUBLIC_DEFAULT_LOCALE', 'en'),
    'cookie' => 'oteryn_locale',
    'cookie_minutes' => 60 * 24 * 365,
    'legacy_public_urls' => 'deterministic_english_compatibility',
    'editorial_fallback' => 'none',
];
