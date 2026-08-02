<?php

namespace App\Payments\Exceptions;

use DomainException;

final class PaymentException extends DomainException
{
    public function __construct(
        public readonly string $reason,
        string $message,
    ) {
        parent::__construct($message);
    }
}
