<?php

namespace App\CharacterAuthority;

/** One decoded `PublishCharacterCommandReceiptV1` body (§4.3). */
final readonly class CharacterCommandReceipt
{
    /** @param array<string, string> $fields committed-only members, keyed by wire name */
    public function __construct(
        public string $sourceAuthority,
        public string $command,
        public string $operationId,
        public string $outcome,
        public string $resultCode,
        public string $recoveryGeneration,
        public string $decidedAt,
        public array $fields,
    ) {}
}
