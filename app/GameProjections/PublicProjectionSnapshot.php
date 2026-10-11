<?php

namespace App\GameProjections;

use InvalidArgumentException;

/**
 * One validated snapshot publication (§4.0-§4.6). The content (subject members, body and, for
 * highscores, `computed_at`) is stored as canonical JSON with sorted object members; its digest
 * decides whether an equal (epoch, revision) pair is an idempotent retry or a `409` conflict.
 */
final readonly class PublicProjectionSnapshot
{
    private const ENVELOPE = [
        'contract_version',
        'operation',
        'source_authority',
        'projection_epoch',
        'projection_revision',
        'source_observed_at',
    ];

    private function __construct(
        public PublicProjectionFamily $family,
        public string $sourceAuthority,
        public string $subjectKey,
        public ?string $worldId,
        public ?string $lookupName,
        public bool $present,
        public string $projectionEpoch,
        public string $projectionRevision,
        public int $sourceObservedAt,
        public string $payload,
        public string $contentDigest,
    ) {}

    public static function fromWire(PublicProjectionFamily $family, string $raw): self
    {
        $decoded = PublicProjectionWire::decodeExact($raw, $family->maxBytes());
        if (($decoded['contract_version'] ?? null) !== 1 || ($decoded['operation'] ?? null) !== $family->operation()) {
            throw new InvalidArgumentException('Public projection envelope is invalid.');
        }

        [$members, $subjectKey, $worldId, $lookupName, $present] = match ($family) {
            PublicProjectionFamily::Highscores => self::highscores($decoded),
            PublicProjectionFamily::CharacterProfile => self::characterProfile($decoded),
            PublicProjectionFamily::CharacterDeaths => self::characterDeaths($decoded),
            PublicProjectionFamily::Guild => self::guild($decoded),
            PublicProjectionFamily::House => self::house($decoded),
            PublicProjectionFamily::WorldOnline => self::worldOnline($decoded),
        };
        PublicProjectionWire::exactKeys($decoded, [...self::ENVELOPE, ...$members]);

        $content = array_diff_key($decoded, array_flip(self::ENVELOPE));
        $payload = json_encode(self::sorted($content), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);

        return new self(
            $family,
            PublicProjectionWire::authority($decoded),
            $subjectKey,
            $worldId,
            $lookupName,
            $present,
            PublicProjectionWire::uint64($decoded, 'projection_epoch'),
            PublicProjectionWire::uint64($decoded, 'projection_revision'),
            PublicProjectionWire::unixTime($decoded, 'source_observed_at'),
            $payload,
            hash('sha256', $payload),
        );
    }

    /**
     * @param  array<string, mixed>  $d
     * @return array{list<string>, string, ?string, ?string, bool}
     */
    private static function highscores(array $d): array
    {
        $worldId = PublicProjectionWire::uuid7($d, 'world_id');
        $category = PublicProjectionWire::int($d, 'category', 1, 11);
        $filter = PublicProjectionWire::int($d, 'filter', 0, 5);
        PublicProjectionWire::bodyTime($d, 'computed_at');

        $seen = [];
        $previousRank = null;
        foreach (PublicProjectionWire::list($d['rows'] ?? null, 0, 1000) as $position => $entry) {
            $row = PublicProjectionWire::object($entry);
            PublicProjectionWire::exactKeys($row, ['rank', 'character_id', 'name', 'vocation', 'level', 'value']);
            // Competition rank in position order (1, 2, 2, 4).
            $rank = PublicProjectionWire::uint64($row, 'rank');
            if ($rank !== (string) ($position + 1) && $rank !== $previousRank) {
                throw new InvalidArgumentException('Public highscores rank is not a competition rank.');
            }
            $characterId = PublicProjectionWire::uuid7($row, 'character_id');
            if (isset($seen[$characterId])) {
                throw new InvalidArgumentException('Public highscores character is listed twice.');
            }
            $seen[$characterId] = true;
            PublicProjectionWire::name($row['name'] ?? null);
            PublicProjectionWire::int($row, 'vocation', 0, 255);
            PublicProjectionWire::uint64($row, 'level');
            PublicProjectionWire::uint64OrZero($row, 'value');
            $previousRank = $rank;
        }

        return [['world_id', 'category', 'filter', 'computed_at', 'rows'], $worldId.':'.$category.':'.$filter, $worldId, null, true];
    }

    /**
     * @param  array<string, mixed>  $d
     * @return array{list<string>, string, ?string, ?string, bool}
     */
    private static function characterProfile(array $d): array
    {
        $characterId = PublicProjectionWire::uuid7($d, 'character_id');
        $worldId = null;
        $name = null;
        foreach (PublicProjectionWire::list($d['profile'] ?? null, 0, 1) as $entry) {
            $profile = PublicProjectionWire::object($entry);
            PublicProjectionWire::exactKeys($profile, ['world_id', 'name', 'vocation', 'level', 'guild']);
            $worldId = PublicProjectionWire::uuid7($profile, 'world_id');
            $name = PublicProjectionWire::name($profile['name'] ?? null);
            PublicProjectionWire::int($profile, 'vocation', 0, 255);
            PublicProjectionWire::uint64($profile, 'level');
            foreach (PublicProjectionWire::list($profile['guild'] ?? null, 0, 1) as $membership) {
                $guild = PublicProjectionWire::object($membership);
                PublicProjectionWire::exactKeys($guild, ['guild_id', 'rank_level']);
                PublicProjectionWire::uuid7($guild, 'guild_id');
                PublicProjectionWire::int($guild, 'rank_level', 1, 20);
            }
        }

        return [['character_id', 'profile'], $characterId, $worldId, $name, $name !== null];
    }

    /**
     * @param  array<string, mixed>  $d
     * @return array{list<string>, string, ?string, ?string, bool}
     */
    private static function characterDeaths(array $d): array
    {
        $characterId = PublicProjectionWire::uuid7($d, 'character_id');
        $previous = null;
        foreach (PublicProjectionWire::list($d['deaths'] ?? null, 0, 20) as $entry) {
            $death = PublicProjectionWire::object($entry);
            PublicProjectionWire::exactKeys($death, ['occurred_at', 'level', 'pvp', 'killers']);
            $occurredAt = PublicProjectionWire::bodyTime($death, 'occurred_at');
            if ($previous !== null && $occurredAt > $previous) {
                throw new InvalidArgumentException('Public deaths are not newest first.');
            }
            $previous = $occurredAt;
            PublicProjectionWire::uint64($death, 'level');
            PublicProjectionWire::bool($death, 'pvp');
            foreach (PublicProjectionWire::list($death['killers'] ?? null, 0, 16) as $killer) {
                PublicProjectionWire::name($killer);
            }
        }

        return [['character_id', 'deaths'], $characterId, null, null, true];
    }

    /**
     * @param  array<string, mixed>  $d
     * @return array{list<string>, string, ?string, ?string, bool}
     */
    private static function guild(array $d): array
    {
        $guildId = PublicProjectionWire::uuid7($d, 'guild_id');
        $worldId = null;
        $name = null;
        foreach (PublicProjectionWire::list($d['guild'] ?? null, 0, 1) as $entry) {
            $guild = PublicProjectionWire::object($entry);
            PublicProjectionWire::exactKeys($guild, ['world_id', 'name', 'state', 'founded_at', 'ranks', 'members']);
            $worldId = PublicProjectionWire::uuid7($guild, 'world_id');
            $name = PublicProjectionWire::name($guild['name'] ?? null);
            PublicProjectionWire::oneOf($guild, 'state', ['FORMING', 'ACTIVE', 'DISBANDING']);
            PublicProjectionWire::bodyTime($guild, 'founded_at');

            $levels = [];
            foreach (PublicProjectionWire::list($guild['ranks'] ?? null, 3, 20) as $rankEntry) {
                $rank = PublicProjectionWire::object($rankEntry);
                PublicProjectionWire::exactKeys($rank, ['level', 'name']);
                $level = PublicProjectionWire::int($rank, 'level', 1, 20);
                if ($levels !== [] && $level <= $levels[array_key_last($levels)]) {
                    throw new InvalidArgumentException('Public guild ranks are not strictly ascending.');
                }
                $levels[] = $level;
                PublicProjectionWire::name($rank['name'] ?? null);
            }

            $previous = null;
            foreach (PublicProjectionWire::list($guild['members'] ?? null, 0, 2000) as $memberEntry) {
                $member = PublicProjectionWire::object($memberEntry);
                PublicProjectionWire::exactKeys($member, ['name', 'rank_level', 'joined_at']);
                $memberName = PublicProjectionWire::name($member['name'] ?? null);
                $rankLevel = PublicProjectionWire::int($member, 'rank_level', 1, 20);
                if (! in_array($rankLevel, $levels, true)) {
                    throw new InvalidArgumentException('Public guild member rank is unknown.');
                }
                if ($previous !== null && ($rankLevel <=> $previous[0] ?: strcmp($memberName, $previous[1]) <=> 0) <= 0) {
                    throw new InvalidArgumentException('Public guild members are not ordered by rank level then name.');
                }
                $previous = [$rankLevel, $memberName];
                PublicProjectionWire::bodyTime($member, 'joined_at');
            }
        }

        return [['guild_id', 'guild'], $guildId, $worldId, $name, $name !== null];
    }

    /**
     * @param  array<string, mixed>  $d
     * @return array{list<string>, string, ?string, ?string, bool}
     */
    private static function house(array $d): array
    {
        $worldId = PublicProjectionWire::uuid7($d, 'world_id');
        $houseKey = PublicProjectionWire::contentKey($d, 'house_key');
        $name = null;
        foreach (PublicProjectionWire::list($d['house'] ?? null, 0, 1) as $entry) {
            $house = PublicProjectionWire::object($entry);
            $status = PublicProjectionWire::oneOf($house, 'status', ['VACANT', 'AUCTION', 'OWNED', 'MOVE_OUT_PENDING', 'UNAVAILABLE']);
            $extra = match ($status) {
                'AUCTION' => ['current_bid', 'auction_ends_at'],
                'OWNED' => ['owner_kind', 'owner_name', 'paid_until'],
                'MOVE_OUT_PENDING' => ['owner_kind', 'owner_name', 'paid_until', 'move_out_at'],
                default => [],
            };
            PublicProjectionWire::exactKeys($house, ['name', 'kind', 'town', 'size_sqm', 'beds', 'rent_gold', 'status', ...$extra]);
            $name = PublicProjectionWire::name($house['name'] ?? null);
            PublicProjectionWire::oneOf($house, 'kind', ['private_house', 'shop', 'guildhall']);
            PublicProjectionWire::contentKey($house, 'town');
            PublicProjectionWire::uint64($house, 'size_sqm');
            PublicProjectionWire::uint64OrZero($house, 'beds');
            PublicProjectionWire::uint64($house, 'rent_gold');
            if ($status === 'AUCTION') {
                PublicProjectionWire::uint64OrZero($house, 'current_bid');
                PublicProjectionWire::bodyTime($house, 'auction_ends_at');
            }
            if ($status === 'OWNED' || $status === 'MOVE_OUT_PENDING') {
                PublicProjectionWire::oneOf($house, 'owner_kind', ['CHARACTER', 'GUILD']);
                PublicProjectionWire::name($house['owner_name'] ?? null);
                PublicProjectionWire::bodyTime($house, 'paid_until');
            }
            if ($status === 'MOVE_OUT_PENDING') {
                PublicProjectionWire::bodyTime($house, 'move_out_at');
            }
        }

        return [['world_id', 'house_key', 'house'], $worldId.':'.$houseKey, $worldId, $name, $name !== null];
    }

    /**
     * @param  array<string, mixed>  $d
     * @return array{list<string>, string, ?string, ?string, bool}
     */
    private static function worldOnline(array $d): array
    {
        $worldId = PublicProjectionWire::uuid7($d, 'world_id');
        $previous = null;
        foreach (PublicProjectionWire::list($d['characters'] ?? null, 0, 4096) as $entry) {
            $character = PublicProjectionWire::object($entry);
            PublicProjectionWire::exactKeys($character, ['name', 'vocation', 'level']);
            $name = PublicProjectionWire::name($character['name'] ?? null);
            if ($previous !== null && strcmp($name, $previous) <= 0) {
                throw new InvalidArgumentException('Public online list is not sorted and unique by name.');
            }
            $previous = $name;
            PublicProjectionWire::int($character, 'vocation', 0, 255);
            PublicProjectionWire::uint64($character, 'level');
        }

        return [['world_id', 'characters'], $worldId, $worldId, null, true];
    }

    /**
     * Sorts object members recursively; list order is content and is kept.
     *
     * @param  array<array-key, mixed>  $value
     * @return array<array-key, mixed>
     */
    private static function sorted(array $value): array
    {
        if (! array_is_list($value)) {
            ksort($value, SORT_STRING);
        }
        foreach ($value as $key => $member) {
            if (is_array($member)) {
                $value[$key] = self::sorted($member);
            }
        }

        return $value;
    }
}
