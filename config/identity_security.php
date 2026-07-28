<?php

return [
    'sessions' => [
        'touch_interval_seconds' => (int) env('IDENTITY_SESSION_TOUCH_INTERVAL_SECONDS', 60),
    ],

    'email_change' => [
        'verification_ttl_hours' => (int) env('IDENTITY_EMAIL_CHANGE_TTL_HOURS', 24),
        'recovery_window_hours' => (int) env('IDENTITY_EMAIL_RECOVERY_WINDOW_HOURS', 24),
        'cooldown_days' => (int) env('IDENTITY_EMAIL_CHANGE_COOLDOWN_DAYS', 7),
    ],

    'termination' => [
        'grace_days' => (int) env('IDENTITY_TERMINATION_GRACE_DAYS', 14),
        'confirmation_phrase' => 'TERMINATE',
    ],

    'recovery_key' => [
        'prefix' => 'OTERYN',
    ],

    'binding_mutation_policy' => 'deny',
    'email_code_mfa_enabled' => false,
];
