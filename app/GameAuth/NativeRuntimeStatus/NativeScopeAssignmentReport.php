<?php

namespace App\GameAuth\NativeRuntimeStatus;

use InvalidArgumentException;

/**
 * One `ReportScopeAssignmentV1` body (Game `oteryn-game-native-runtime-status-v1` §5): the committed
 * #415 assignment of a scope to a node-host runtime-status identity, reported by the scope ownership
 * authority. Exactly 8 members in the Game grammar; uint64 values stay canonical decimal strings.
 */
final readonly class NativeScopeAssignmentReport
{
    private const UUID7 = '/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/D';

    /** The runtime-status identity grammar (a configured certificate subject). */
    public const IDENTITY = '/^[\x20-\x7e]{1,128}$/D';

    private function __construct(
        public string $worldId,
        public string $channelId,
        public string $assignmentEpoch,
        public string $ownershipGeneration,
        public string $nodeIdentity,
        public int $assignedAt,
    ) {}

    public static function fromWire(string $raw): self
    {
        $decoded = NativeRuntimeStatusReport::decode($raw, 8, 'ReportScopeAssignmentV1');
        $text = static fn (string $name, string $grammar): string => is_string($value = NativeRuntimeStatusReport::member($decoded, $name, $grammar))
            ? $value
            : throw new InvalidArgumentException('Scope assignment report member is invalid.');
        $assignedAt = NativeRuntimeStatusReport::member($decoded, 'assigned_at', 'time');

        return new self(
            $text('world_id', self::UUID7),
            $text('channel_id', self::UUID7),
            $text('assignment_epoch', 'uint64'),
            $text('ownership_generation', 'uint64'),
            $text('node_identity', self::IDENTITY),
            is_int($assignedAt) ? $assignedAt : throw new InvalidArgumentException('Scope assignment time is invalid.'),
        );
    }
}
