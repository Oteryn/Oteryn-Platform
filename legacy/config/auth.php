<?php

use App\Identity\Models\Identity;

return [
    'defaults' => [
        'guard' => env('AUTH_GUARD', 'web'),
        'passwords' => env('AUTH_PASSWORD_BROKER', 'identities'),
    ],

    'guards' => [
        'web' => [
            'driver' => 'session',
            'provider' => 'identities',
        ],
        'api' => [
            'driver' => 'passport',
            'provider' => 'identities',
        ],
    ],

    'providers' => [
        'identities' => [
            'driver' => 'eloquent',
            'model' => Identity::class,
        ],
    ],

    'passwords' => [
        'identities' => [
            'provider' => 'identities',
            'table' => env('AUTH_PASSWORD_RESET_TOKEN_TABLE', 'password_reset_tokens'),
            'expire' => 60,
            'throttle' => 60,
        ],
    ],

    'password_timeout' => env('AUTH_PASSWORD_TIMEOUT', 10800),
];
