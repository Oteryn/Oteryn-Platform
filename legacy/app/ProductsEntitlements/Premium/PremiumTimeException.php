<?php

namespace App\ProductsEntitlements\Premium;

use RuntimeException;

/** A refused operator grant or revocation. `reason` is a stable machine code; the message is operator-safe. */
final class PremiumTimeException extends RuntimeException
{
    public function __construct(public readonly string $reason, string $message)
    {
        parent::__construct($message);
    }
}
