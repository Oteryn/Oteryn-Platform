<?php

return [
    // Game Public Projections v1 consumer. It accepts publications only in testing/preproduction
    // and only while enabled; it is off by default everywhere.
    'public' => [
        'enabled' => env('GAME_PROJECTIONS_PUBLIC_ENABLED', false),
        // JSON list of dedicated public-projection certificate subjects, disjoint from every
        // other internal mTLS identity.
        'identities' => env('GAME_PROJECTIONS_PUBLIC_IDENTITIES'),
        // The one expected Game source authority namespace.
        'source_authority' => env('GAME_PROJECTIONS_PUBLIC_SOURCE_AUTHORITY'),
        'requests_per_minute' => env('GAME_PROJECTIONS_PUBLIC_REQUESTS_PER_MINUTE', 1200),
    ],
];
