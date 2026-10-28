<?php

return [
    // Store wire §11: default off and limited to testing/preproduction by StoreSettings.
    'store' => [
        'purchase_enabled' => env('PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED', false),
        'delivery_enabled' => env('PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED', false),
        'enabled_unit_kinds' => env('PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS', []),
        'mtls_client_identity' => env('PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY'),
        'source_authority' => env('PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY'),
        'requests_per_minute' => env('PRODUCTS_ENTITLEMENTS_STORE_REQUESTS_PER_MINUTE', 600),
        'purchases_per_account_per_minute' => env('PRODUCTS_ENTITLEMENTS_STORE_PURCHASES_PER_ACCOUNT_PER_MINUTE', 10),
        'reads_per_account_per_minute' => env('PRODUCTS_ENTITLEMENTS_STORE_READS_PER_ACCOUNT_PER_MINUTE', 60),
        'max_pending_operations' => env('PRODUCTS_ENTITLEMENTS_STORE_MAX_PENDING_OPERATIONS', 1000),
    ],

    // oteryn.premium_time v1 snapshot read; lease policy is fixed in PremiumTimeContract.
    'premium_snapshot' => [
        'enabled' => env('PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED', false),
        // Verified client-certificate subject dedicated to this one read; never another internal purpose's identity.
        'mtls_client_identity' => env('PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY'),
        'requests_per_minute' => env('PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_REQUESTS_PER_MINUTE', '1200'),
        // 40-character lower-case commit SHA of the running build, echoed as producer_revision.
        'producer_revision' => env('PLATFORM_BUILD_REVISION'),
    ],
];
