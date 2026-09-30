<?php

namespace App\GameAuth\NativeRuntimeStatus;

/**
 * Validated `game-auth.native_scope_assignment` configuration. `current()` is null while the default-off
 * switch is off or any value is invalid (fail closed, 503). Each identity is a scope ownership authority
 * certificate subject (`oteryn-game-ops`) with the scopes whose assignments it may report; it may never
 * equal another purpose's identity, including any runtime-status identity.
 */
final readonly class NativeScopeAssignmentSettings
{
    /** @param array<string, list<string>> $identities */
    private function __construct(private array $identities, public int $requestsPerMinute) {}

    public static function current(): ?self
    {
        if (config('game-auth.native_scope_assignment.enabled') !== true) {
            return null;
        }
        $identities = NativeRuntimeStatusSettings::identities(config('game-auth.native_scope_assignment.identities'), 'native_runtime_status');
        $rate = NativeRuntimeStatusSettings::bounded(config('game-auth.native_scope_assignment.requests_per_minute'), 1, 600);

        return $identities === null || $rate === null ? null : new self($identities, $rate);
    }

    public function knows(string $identity): bool
    {
        return isset($this->identities[$identity]);
    }

    public function allows(string $identity, string $worldId, string $channelId): bool
    {
        return in_array($worldId.'/'.$channelId, $this->identities[$identity] ?? [], true);
    }
}
