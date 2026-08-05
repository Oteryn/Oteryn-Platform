<?php

namespace App\Payments\Data;

final readonly class PaymentStateDecision
{
    public const APPLY = 'apply';

    public const NOOP = 'noop';

    public const RECONCILE = 'reconcile';

    public function __construct(
        public string $action,
        public string $currentStatus,
        public ?string $targetStatus,
        public string $reason,
    ) {}
}
