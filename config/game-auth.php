<?php

$nativeEvidenceClockUncertainty = env('GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS');
$nativeEvidenceActivated = env('GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED', false);
$nativeEvidenceRequestsPerMinute = env('GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE', '120');

return [
    'protocol_version' => 1,

    'oauth' => [
        'native_client_name' => env('GAME_AUTH_OAUTH_NATIVE_CLIENT_NAME', 'Oteryn OTClient'),
        'native_redirect_uri' => env('GAME_AUTH_OAUTH_NATIVE_REDIRECT_URI', 'http://127.0.0.1/callback'),
        'scope' => 'game:ticket',
        'access_token_ttl_minutes' => (int) env('GAME_AUTH_OAUTH_ACCESS_TOKEN_TTL_MINUTES', 5),
        'refresh_token_ttl_minutes' => (int) env('GAME_AUTH_OAUTH_REFRESH_TOKEN_TTL_MINUTES', 10),
    ],

    'ticket' => [
        'audience' => 'oteryn-game-gateway',
        'ttl_seconds' => (int) env('GAME_AUTH_TICKET_TTL_SECONDS', 60),
    ],

    'gateway' => [
        'service_token_sha256' => env('GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256'),
        'previous_service_token_sha256' => env('GAME_AUTH_GATEWAY_PREVIOUS_SERVICE_TOKEN_SHA256'),
    ],

    'native_evidence' => [
        'source_authority' => env('GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY', 'platform'),
        'activated' => $nativeEvidenceActivated,
        'mtls_client_identity' => env('GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY'),
        'high_water_directory' => env('GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY'),
        'fresh_account_purpose' => env('GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE'),
        'fresh_account_scope' => env('GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE'),
        'fresh_key_purpose' => env('GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE'),
        'clock_uncertainty_seconds' => $nativeEvidenceClockUncertainty,
        'requests_per_minute' => $nativeEvidenceRequestsPerMinute,
    ],
];
