<?php

namespace App\GameAuth\Worlds;

use LogicException;

/**
 * Immutable identity-only readback. Possession is not custody, readiness,
 * admission, current assignment or authorization evidence.
 */
final readonly class NativeTopologyReceipt
{
    public function __construct(
        public string $worldId,
        public string $channelId,
    ) {
        if (! self::isCanonicalId($worldId) || ! self::isCanonicalId($channelId) || $worldId === $channelId) {
            throw new LogicException('Native topology requires distinct canonical UUIDv7 WorldId and ChannelId.');
        }
    }

    public static function isCanonicalId(mixed $value): bool
    {
        return is_string($value)
            && preg_match('/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/', $value) === 1;
    }

    /** @return array{version:int,purpose:string,issuer:string,world_id:string,channel_id:string} */
    public function toArray(): array
    {
        return [
            'version' => 1,
            'purpose' => 'disposable-preproduction-native-topology',
            'issuer' => 'oteryn-platform-world-registry',
            'world_id' => $this->worldId,
            'channel_id' => $this->channelId,
        ];
    }
}
