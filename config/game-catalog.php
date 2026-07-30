<?php

return [
    'contract' => 'oteryn.game-catalog',
    'schemas' => [
        '1.0.0' => [
            'path' => resource_path('schemas/game-catalog/v1/game-catalog-snapshot.schema.json'),
            'sha256' => '099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b',
            'activatable' => true,
        ],
        '1.1.0' => [
            'path' => resource_path('schemas/game-catalog/v1.1/game-catalog-snapshot.schema.json'),
            'sha256' => '323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac',
            'activatable' => true,
        ],
        '1.2.0' => [
            'path' => resource_path('schemas/game-catalog/v1.2/game-catalog-snapshot.schema.json'),
            'sha256' => 'a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de',
            'activatable' => true,
        ],
        '1.3.0' => [
            'path' => resource_path('schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json'),
            'sha256' => 'f0241a36b579cc29fb7ab99397bfbce2ead90f78e1045bfd56030620ea7be8fe',
            'activatable' => false,
        ],
    ],

    'limits' => [
        'file_bytes' => 256 * 1024 * 1024,
        'releases' => 512,
        'entities' => 200_000,
        'relations' => 1_000_000,
        'identifiers_per_entity' => 32,
        'string_bytes' => 2_000,
        'source_path_bytes' => 512,
        'validation_findings' => 2_000,
    ],

    'public_item_availability' => [
        'obtainable',
        'quest_only',
        'boss_only',
        'event_only',
        'npc_only',
        'starter',
    ],

    'public_creature_availability' => [
        'encounterable',
        'boss_only',
        'event_only',
        'quest_only',
    ],

    'public_completeness' => [
        'complete',
    ],

    'pagination' => [
        'default' => 30,
        'maximum' => 100,
    ],
];
