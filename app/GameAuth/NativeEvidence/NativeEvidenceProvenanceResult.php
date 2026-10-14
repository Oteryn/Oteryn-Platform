<?php

namespace App\GameAuth\NativeEvidence;

/** Outcome of NativeEvidenceHighWaterWitness::initializeStoreProvenance(); refusals throw instead. */
final class NativeEvidenceProvenanceResult
{
    public const CREATED = 'created';

    public const COMPLETED = 'completed';

    public const UNCHANGED = 'unchanged';

    public const BUSY = 'busy';

    /** @param array<string, int> $entries Count of directory entries per class (design note D1). */
    public function __construct(
        public readonly string $result,
        public readonly ?string $storeId,
        public readonly array $entries,
    ) {}

    public function succeeded(): bool
    {
        return $this->result !== self::BUSY;
    }

    /** @return array{result: string, store_id: string|null, entries: array<string, int>} */
    public function toArray(): array
    {
        return ['result' => $this->result, 'store_id' => $this->storeId, 'entries' => $this->entries];
    }
}
