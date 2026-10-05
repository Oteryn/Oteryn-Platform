<?php

namespace App\GameAuth\NativeRuntimeStatus;

/**
 * Public-safe view of one scope's accepted runtime evidence. It deliberately omits GameNode identity,
 * assignment/fencing generations, route revisions and every private topology field.
 */
final readonly class NativeRuntimePublicEvidence
{
    public function __construct(
        public string $state,
        public ?bool $ready,
        public ?int $observedAt,
    ) {}
}
