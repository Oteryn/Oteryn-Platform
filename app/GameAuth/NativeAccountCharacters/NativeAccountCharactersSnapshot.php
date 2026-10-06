<?php

namespace App\GameAuth\NativeAccountCharacters;

use InvalidArgumentException;

final readonly class NativeAccountCharactersSnapshot
{
    /**
     * @param  list<NativeAccountCharacterSummary>  $characters
     */
    private function __construct(
        public string $sourceAuthority,
        public string $accountId,
        public string $projectionEpoch,
        public string $projectionRevision,
        public int $sourceObservedAt,
        public array $characters,
        public string $contentDigest,
    ) {}

    public static function fromWire(string $raw): self
    {
        $decoded = NativeAccountCharactersWire::decodeExact($raw, NativeAccountCharactersWire::SNAPSHOT_MAX_BYTES);
        NativeAccountCharactersWire::exactKeys($decoded, [
            'contract_version',
            'operation',
            'source_authority',
            'account_id',
            'projection_epoch',
            'projection_revision',
            'source_observed_at',
            'characters',
        ]);
        if (($decoded['contract_version'] ?? null) !== 1 || ($decoded['operation'] ?? null) !== 'PublishAccountCharactersV1') {
            throw new InvalidArgumentException('Native account-character snapshot envelope is invalid.');
        }

        $rawCharacters = $decoded['characters'] ?? null;
        if (! is_array($rawCharacters) || ! array_is_list($rawCharacters) || count($rawCharacters) > 64) {
            throw new InvalidArgumentException('Native account-character list is invalid.');
        }

        $characters = [];
        $canonicalCharacters = [];
        $previous = null;
        foreach ($rawCharacters as $entry) {
            if (! is_array($entry) || array_is_list($entry)) {
                throw new InvalidArgumentException('Native account-character entry is invalid.');
            }
            /** @var array<string, mixed> $entry */
            NativeAccountCharactersWire::exactKeys($entry, ['character_id', 'world_id', 'name', 'availability']);
            $characterId = NativeAccountCharactersWire::uuid7($entry, 'character_id');
            if ($previous !== null && strcmp($previous, $characterId) >= 0) {
                throw new InvalidArgumentException('Native account-character entries are not strictly sorted.');
            }
            $worldId = NativeAccountCharactersWire::uuid7($entry, 'world_id');
            $name = NativeAccountCharactersWire::characterName($entry['name'] ?? null);
            $availability = $entry['availability'] ?? null;
            if ($availability !== 'AVAILABLE' && $availability !== 'UNAVAILABLE') {
                throw new InvalidArgumentException('Native account-character availability is invalid.');
            }

            $previous = $characterId;
            $characters[] = new NativeAccountCharacterSummary($characterId, $worldId, $name, $availability);
            $canonicalCharacters[] = [
                'character_id' => $characterId,
                'world_id' => $worldId,
                'name' => $name,
                'availability' => $availability,
            ];
        }

        $content = json_encode($canonicalCharacters, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);

        return new self(
            NativeAccountCharactersWire::authority($decoded),
            NativeAccountCharactersWire::uuid7($decoded, 'account_id'),
            NativeAccountCharactersWire::uint64($decoded, 'projection_epoch'),
            NativeAccountCharactersWire::uint64($decoded, 'projection_revision'),
            NativeAccountCharactersWire::unixTime($decoded, 'source_observed_at'),
            $characters,
            hash('sha256', $content),
        );
    }
}
