<?php

namespace App\ProductsEntitlements\Store\Delivery;

use RuntimeException;

final class StoreDeliveryRefused extends RuntimeException
{
    public function __construct(public readonly int $status)
    {
        parent::__construct('Store delivery exchange refused the request.');
    }
}
