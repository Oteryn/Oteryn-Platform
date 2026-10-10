<?php

namespace App\GameAuth\NativeAccountCharacters;

use InvalidArgumentException;

final readonly class NativeAccountCharactersWatermark
{
    private function __construct(
        public string $sourceAuthority,
        public string $projectionEpoch,
        public int $completeThrough,
        public int $observedAt,
    ) {}

    public static function fromWire(string $raw): self
    {
        $decoded = NativeAccountCharactersWire::decodeExact($raw, NativeAccountCharactersWire::WATERMARK_MAX_BYTES);
        NativeAccountCharactersWire::exactKeys($decoded, [
            'contract_version',
            'operation',
            'source_authority',
            'projection_epoch',
            'complete_through',
            'observed_at',
        ]);
        if (($decoded['contract_version'] ?? null) !== 1 || ($decoded['operation'] ?? null) !== 'PublishProjectionWatermarkV1') {
            throw new InvalidArgumentException('Native account-character watermark envelope is invalid.');
        }

        $complete = NativeAccountCharactersWire::unixTime($decoded, 'complete_through');
        $observed = NativeAccountCharactersWire::unixTime($decoded, 'observed_at');
        if ($complete > $observed) {
            throw new InvalidArgumentException('Native account-character watermark completes after observation.');
        }

        return new self(
            NativeAccountCharactersWire::authority($decoded),
            NativeAccountCharactersWire::uint64($decoded, 'projection_epoch'),
            $complete,
            $observed,
        );
    }
}
