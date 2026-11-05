<?php

namespace App\GameAuth\NativeRuntimeStatus;

use InvalidArgumentException;

/** One committed scope revocation from Game `oteryn-game-native-runtime-status-v1` §16.1. */
final readonly class NativeScopeRevocationReport
{
    private const UUID7 = '/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/D';

    private function __construct(
        public string $worldId,
        public string $channelId,
        public string $assignmentEpoch,
        public string $ownershipGeneration,
        public int $revokedAt,
    ) {}

    public static function fromWire(string $raw): self
    {
        $decoded = NativeRuntimeStatusReport::decode($raw, 7, 'ReportScopeRevocationV1');
        $text = static fn (string $name, string $grammar): string => is_string($value = NativeRuntimeStatusReport::member($decoded, $name, $grammar))
            ? $value
            : throw new InvalidArgumentException('Scope revocation report member is invalid.');
        $worldId = $text('world_id', self::UUID7);
        $channelId = $text('channel_id', self::UUID7);
        $revokedAt = NativeRuntimeStatusReport::member($decoded, 'revoked_at', 'time');
        if ($worldId === $channelId) {
            throw new InvalidArgumentException('Scope revocation world and channel must differ.');
        }

        return new self(
            $worldId,
            $channelId,
            $text('assignment_epoch', 'uint64'),
            $text('ownership_generation', 'uint64'),
            is_int($revokedAt) ? $revokedAt : throw new InvalidArgumentException('Scope revocation time is invalid.'),
        );
    }
}
