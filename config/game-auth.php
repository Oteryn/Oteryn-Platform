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

    'native_admission' => [
        // Default off. Testing/preproduction only (N4P-1); production needs separate authority (U8, U9).
        'enabled' => env('GAME_AUTH_NATIVE_ADMISSION_ENABLED', false),
        // Paths to externally injected secret files holding the base64url Ed25519 seed; never the key itself.
        // Each file must be a regular, non-symlink file owned by the issuer process user with mode 0600
        // or stricter. Kubernetes secret volumes expose keys as symlinks and are refused: mount the key
        // at a non-symlink path (for example a subPath mount). Production custody stays U9.
        'signing_key_file' => env('GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE'),
        'signing_key_id' => env('GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID'),
        'retiring_signing_key_file' => env('GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_FILE'),
        'retiring_signing_key_id' => env('GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_ID'),
        'grant_ttl_seconds' => env('GAME_AUTH_NATIVE_ADMISSION_GRANT_TTL_SECONDS', 20),
    ],

    'native_runtime_status' => [
        // Default off (N4P rollout step 4). While off, ReportRuntimeStatusV1 answers 503, stores nothing and no scope routes.
        'enabled' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED', false),
        // JSON object: runtime-status client certificate subject => ["<world_id>/<channel_id>", ...] it may serve.
        // Never another purpose's identity (native evidence, character bootstrap); U15 PKI is not decided.
        'identities' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES'),
        // F and Platform clock uncertainty (U5; Game heartbeat H = 5 s).
        'freshness_seconds' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_FRESHNESS_SECONDS', 15),
        'clock_uncertainty_seconds' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_CLOCK_UNCERTAINTY_SECONDS', 1),
        'requests_per_minute' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_REQUESTS_PER_MINUTE', 600),
    ],

    'character_bootstrap_intent' => [
        'ttl_seconds' => env('GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS'),
        'mtls_client_identity' => env('GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY'),
    ],
];
