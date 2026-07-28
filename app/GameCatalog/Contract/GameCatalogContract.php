<?php

namespace App\GameCatalog\Contract;

final class GameCatalogContract
{
    public const ID = 'oteryn.game-catalog';

    public const SCHEMA_VERSION = '1.0.0';

    public const SCHEMA_SHA256 = '099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b';

    public const SCHEMA_PATH = 'resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json';

    public const MAX_DOCUMENT_BYTES = 67_108_864;

    public const MAX_VALIDATION_FINDINGS = 200;

    private function __construct() {}
}
