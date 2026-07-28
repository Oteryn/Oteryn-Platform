<?php

return [
    'contract' => 'oteryn.game-catalog',
    'schema_version' => '1.0.0',
    'schema_path' => resource_path('schemas/game-catalog/v1/game-catalog-snapshot.schema.json'),
    'expected_schema_sha256' => '099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b',

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
