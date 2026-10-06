<?php

$nativeEvidenceClockUncertainty = env('GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS');
$nativeEvidenceActivated = env('GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED', false);
$nativeEvidenceRequestsPerMinute = env('GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE', '120');
$acceptanceNativeRuntime = env('APP_ENV') === 'acceptance';
$acceptanceNativeRuntimeScope = '018f0f1e-7b2c-7a31-8d4e-1234567890ab/018f0f1e-7b2c-7a32-8d4e-1234567890ac';
$acceptanceNativeRuntimeIdentities = $acceptanceNativeRuntime
    ? ['CN=acceptance-runtime-node' => [$acceptanceNativeRuntimeScope]]
    : null;

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
        // D171, testing/preproduction only: issue without verifying AccountId -> CharacterId ownership for
        // Characters of the one configured World; Game FND-04A §5 admission stays the fail-closed guard.
        // Refused (NATIVE_LOGIN_UNAVAILABLE) in any other environment. Release gate: CHAR-NAME-1 -> LCFA-1 + §5.4.
        'unverified_character_ownership' => env('GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP', false),
        'unverified_character_world_id' => env('GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID'),
        // InnoDB lock wait bound for the issuer transaction (1..10 s); a timeout rolls back and fails closed.
        'lock_wait_timeout_seconds' => env('GAME_AUTH_NATIVE_ADMISSION_LOCK_WAIT_TIMEOUT_SECONDS', 3),
        // Issuer requests per minute per Gateway service credential (contract §10).
        'requests_per_minute' => env('GAME_AUTH_NATIVE_ADMISSION_REQUESTS_PER_MINUTE', 120),
    ],

    'native_runtime_status' => [
        // Default off (N4P rollout step 4). While off, ReportRuntimeStatusV1 answers 503, stores nothing and no scope routes.
        'enabled' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED', $acceptanceNativeRuntime),
        // JSON object: runtime-status client certificate subject => ["<world_id>/<channel_id>", ...] it may serve.
        // Never another purpose's identity (native evidence, character bootstrap); U15 PKI is not decided.
        // The acceptance-only default binds one deterministic isolated fixture; every other environment remains off by default.
        'identities' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES', $acceptanceNativeRuntimeIdentities),
        // F and Platform clock uncertainty (U5; Game heartbeat H = 5 s).
        'freshness_seconds' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_FRESHNESS_SECONDS', 15),
        'clock_uncertainty_seconds' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_CLOCK_UNCERTAINTY_SECONDS', 1),
        'requests_per_minute' => env('GAME_AUTH_NATIVE_RUNTIME_STATUS_REQUESTS_PER_MINUTE', 600),
    ],

    'native_scope_assignment' => [
        // Default off. While off (or while native_runtime_status is invalid), ReportScopeAssignmentV1 answers 503.
        'enabled' => env('GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED', false),
        // JSON object: scope ownership authority (oteryn-game-ops) certificate subject => ["<world_id>/<channel_id>", ...]
        // whose assignments it may report. Never another purpose's identity, including a runtime-status identity.
        'identities' => env('GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES'),
        'requests_per_minute' => env('GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_REQUESTS_PER_MINUTE', 120),
    ],

    'native_account_characters' => [
        // Testing/preproduction push consumer only. Production remains blocked on Decision P1/U12.
        'enabled' => env('GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED', false),
        // JSON object: dedicated Character Authority projection certificate subject => source_authority namespace.
        'publishers' => env('GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_PUBLISHERS'),
        // Accepted internal-build liveness bound; release acceptance remains a separate decision.
        'freshness_seconds' => env('GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_FRESHNESS_SECONDS', 30),
        'clock_uncertainty_seconds' => env('GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_CLOCK_UNCERTAINTY_SECONDS', 1),
        'requests_per_minute' => env('GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_REQUESTS_PER_MINUTE', 120),
    ],

    'character_bootstrap_intent' => [
        'ttl_seconds' => env('GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS'),
        'mtls_client_identity' => env('GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY'),
    ],
];
