<?php

namespace App\GameProjections;

/**
 * The closed family set of Game Public Projections v1 (Game contract
 * `oteryn-game-public-projections-v1` §2.1, §3, §8). Each family has one compiled ingest path.
 */
enum PublicProjectionFamily: string
{
    case Highscores = 'highscores';
    case CharacterProfile = 'character_profile';
    case CharacterDeaths = 'character_deaths';
    case Guild = 'guild';
    case House = 'house';
    case WorldOnline = 'world_online';

    public const WATERMARK_PATH = 'watermark';

    public const WATERMARK_OPERATION = 'PublishPublicProjectionWatermarkV1';

    public const WATERMARK_MAX_BYTES = 512;

    public function path(): string
    {
        return str_replace('_', '-', $this->value);
    }

    public function operation(): string
    {
        return match ($this) {
            self::Highscores => 'PublishWorldHighscoresV1',
            self::CharacterProfile => 'PublishCharacterProfileV1',
            self::CharacterDeaths => 'PublishCharacterDeathsV1',
            self::Guild => 'PublishGuildV1',
            self::House => 'PublishHouseV1',
            self::WorldOnline => 'PublishWorldOnlineV1',
        };
    }

    /** Request byte bound (§8 `PUBPROJ-*-REQUEST-BYTES`). */
    public function maxBytes(): int
    {
        return match ($this) {
            self::Highscores, self::Guild => 262144,
            self::CharacterProfile, self::House => 2048,
            self::CharacterDeaths => 32768,
            self::WorldOnline => 524288,
        };
    }
}
