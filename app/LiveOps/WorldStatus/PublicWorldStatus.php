<?php

namespace App\LiveOps\WorldStatus;

final readonly class PublicWorldStatus
{
    public const POLICY_ONLINE = 'online';

    public const POLICY_MAINTENANCE = 'maintenance';

    public const POLICY_OFFLINE = 'offline';

    public const POLICY_DISABLED = 'disabled';

    public const POLICY_UNKNOWN = 'unknown';

    public const RUNTIME_READY = 'ready';

    public const RUNTIME_NOT_READY = 'not_ready';

    public const RUNTIME_DEGRADED = 'degraded';

    public const RUNTIME_STALE = 'stale';

    public const RUNTIME_UNAVAILABLE = 'unavailable';

    public const RUNTIME_INVALID = 'invalid';

    public function __construct(
        public string $worldId,
        public string $slug,
        public string $name,
        public string $policyState,
        public string $runtimeState,
        public ?int $observedAt,
    ) {}

    public function publicState(): string
    {
        return match ($this->policyState) {
            self::POLICY_MAINTENANCE => 'maintenance',
            self::POLICY_OFFLINE => 'policy_offline',
            self::POLICY_DISABLED => 'login_disabled',
            self::POLICY_UNKNOWN => 'policy_unknown',
            default => $this->runtimeState,
        };
    }

    public function isPartial(): bool
    {
        return in_array($this->publicState(), [
            self::RUNTIME_DEGRADED,
            self::RUNTIME_STALE,
            self::RUNTIME_UNAVAILABLE,
            self::RUNTIME_INVALID,
            'policy_unknown',
        ], true);
    }
}
