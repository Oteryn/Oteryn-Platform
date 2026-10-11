<?php

namespace App\GameAuth\Worlds;

use LogicException;

/**
 * A named refusal of the preproduction-store mode of native topology issuance and route publication
 * (ADR 0044). The reason is one of the constants; the message is a fixed diagnostic that never carries
 * a credential, connection string or database host.
 */
final class NativePreproductionStoreRefused extends LogicException
{
    public const SWITCH_INVALID = 'NATIVE_PREPROD_STORE_SWITCH_INVALID';

    public const NOT_ENABLED = 'NATIVE_PREPROD_STORE_NOT_ENABLED';

    public const ENVIRONMENT = 'NATIVE_PREPROD_STORE_ENVIRONMENT_REFUSED';

    public const WORLD_UNCONFIGURED = 'NATIVE_PREPROD_STORE_WORLD_UNCONFIGURED';

    public const TRANSACTION_OPEN = 'NATIVE_PREPROD_STORE_TRANSACTION_OPEN';

    public const DATABASE = 'NATIVE_PREPROD_STORE_DATABASE_REFUSED';

    public const WORLD_ROW_MISSING = 'NATIVE_PREPROD_STORE_WORLD_ROW_MISSING';

    public const WORLD_MISMATCH = 'NATIVE_PREPROD_STORE_WORLD_MISMATCH';

    public const FOREIGN_WORLD = 'NATIVE_PREPROD_STORE_FOREIGN_WORLD';

    public const CHANNEL_NOT_ISSUED = 'NATIVE_PREPROD_STORE_CHANNEL_NOT_ISSUED';

    public const FOREIGN_ROUTE = 'NATIVE_PREPROD_STORE_FOREIGN_ROUTE';

    public function __construct(public readonly string $reason, string $message)
    {
        parent::__construct($message);
    }
}
