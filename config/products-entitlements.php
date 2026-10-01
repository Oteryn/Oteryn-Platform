<?php

return [
    // oteryn.premium_time v1 snapshot read (docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md section 4).
    // Product policy values (lease, refresh, skew, stale use) are fixed in PremiumTimeContract, not configured here.
    'premium_snapshot' => [
        'enabled' => env('PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED', false),
        // Verified client-certificate subject dedicated to this one read; never another internal purpose's identity.
        'mtls_client_identity' => env('PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY'),
        'requests_per_minute' => env('PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_REQUESTS_PER_MINUTE', '1200'),
        // 40-character lower-case commit SHA of the running build, echoed as producer_revision.
        'producer_revision' => env('PLATFORM_BUILD_REVISION'),
    ],
];
