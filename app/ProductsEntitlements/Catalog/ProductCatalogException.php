<?php

namespace App\ProductsEntitlements\Catalog;

use RuntimeException;
use Throwable;

final class ProductCatalogException extends RuntimeException
{
    public function __construct(
        public readonly string $reason,
        string $message,
        ?Throwable $previous = null,
    ) {
        parent::__construct($message, 0, $previous);
    }
}
