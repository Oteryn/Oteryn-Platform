<?php

namespace App\GameCatalog\Application\Import\Native;

final class NativeCatalogContract
{
    public const string CONTRACT_ID = 'oteryn.game-platform-catalog';

    public const string SCHEMA_VERSION = '1.0.0';

    public const string CONTENT_AUTHORITY_ID = 'oteryn-native';

    public const string CANONICAL_REPOSITORY = 'Oteryn/Oteryn-Game';

    public const string CANONICAL_COMMIT = '96ea673839f1d93190a40c17ae8036ac82096ded';

    public const string CONTRACT_SHA256 = '9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad';

    public const int MAX_FILE_BYTES = 268_435_456;

    public const int MAX_CAPABILITIES = 256;

    public const int MAX_ENTITIES = 200_000;

    public const int MAX_RELATIONS = 1_000_000;

    public const int MAX_TOMBSTONES = 200_000;

    public const int MAX_STRING_BYTES = 2_048;

    public const int MAX_DATA_DEPTH = 16;

    public const int MAX_OBJECT_MEMBERS = 4_096;

    public const int MAX_ARRAY_ENTRIES = 200_000;

    public const int JSON_DECODE_DEPTH = 32;

    /** @var list<string> */
    public const array KNOWN_CAPABILITIES = [
        'achievement',
        'creature',
        'creature_loot',
        'item',
        'npc',
        'npc_shop',
        'quest',
        'spell',
    ];
}
