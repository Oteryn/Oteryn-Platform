<?php

namespace App\ProductsEntitlements\Loyalty;

/** Fixed wire values for the default-off loyalty snapshot producer. */
final class LoyaltyContract
{
    public const REQUEST_SCHEMA = 'oteryn.loyalty_snapshot_request.v1';

    public const SNAPSHOT_SCHEMA = 'oteryn.loyalty_snapshot.v1';

    public const MAX_REQUEST_BYTES = 256;

    public const MAX_RESPONSE_BYTES = 512;

    public const MAX_POINTS = 2147483647;

    public const SECONDS_PER_POINT = 86400;
}
