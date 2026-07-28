<?php

namespace App\Marketplace\Exceptions;

use DomainException;

class MarketplaceException extends DomainException
{
    public function __construct(
        public readonly string $reason,
        string $message,
    ) {
        parent::__construct($message);
    }
}
