<?php

namespace App\GameProjections;

use InvalidArgumentException;

/** Liveness watermark of one (source_authority, family) feed (§5.1). */
final readonly class PublicProjectionWatermark
{
    private function __construct(
        public string $sourceAuthority,
        public PublicProjectionFamily $family,
        public string $projectionEpoch,
        public int $completeThrough,
        public int $observedAt,
    ) {}

    public static function fromWire(string $raw): self
    {
        $decoded = PublicProjectionWire::decodeExact($raw, PublicProjectionFamily::WATERMARK_MAX_BYTES);
        PublicProjectionWire::exactKeys($decoded, [
            'contract_version',
            'operation',
            'source_authority',
            'family',
            'projection_epoch',
            'complete_through',
            'observed_at',
        ]);
        if (($decoded['contract_version'] ?? null) !== 1 || ($decoded['operation'] ?? null) !== PublicProjectionFamily::WATERMARK_OPERATION) {
            throw new InvalidArgumentException('Public projection watermark envelope is invalid.');
        }
        $family = is_string($decoded['family'] ?? null) ? PublicProjectionFamily::tryFrom($decoded['family']) : null;
        if ($family === null) {
            throw new InvalidArgumentException('Public projection watermark family is invalid.');
        }

        $complete = PublicProjectionWire::unixTime($decoded, 'complete_through');
        $observed = PublicProjectionWire::unixTime($decoded, 'observed_at');
        if ($complete > $observed) {
            throw new InvalidArgumentException('Public projection watermark completes after observation.');
        }

        return new self(
            PublicProjectionWire::authority($decoded),
            $family,
            PublicProjectionWire::uint64($decoded, 'projection_epoch'),
            $complete,
            $observed,
        );
    }
}
