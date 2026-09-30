<?php

namespace App\GameAuth\NativeRuntimeStatus;

use JsonException;

/**
 * Validated `game-auth.native_runtime_status` configuration. `current()` is null while the default-off
 * switch is off or any value is invalid, so ingestion answers 503 and nothing routes (fail closed).
 * Each runtime-status identity is one node host's certificate subject with the scopes it may serve
 * (login contract §7.2); it may never equal another purpose's identity, including an ownership-authority
 * identity (`native_scope_assignment`).
 */
final readonly class NativeRuntimeStatusSettings
{
    private const SCOPE = '/^([0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12})\/(?1)$/D';

    /** @param array<string, list<string>> $identities */
    private function __construct(
        private array $identities,
        public int $freshnessSeconds,
        public int $clockUncertaintySeconds,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('game-auth.native_runtime_status.enabled') !== true) {
            return null;
        }
        $identities = self::identities(config('game-auth.native_runtime_status.identities'), 'native_scope_assignment');
        $freshness = self::bounded(config('game-auth.native_runtime_status.freshness_seconds'), 1, 60);
        $uncertainty = self::bounded(config('game-auth.native_runtime_status.clock_uncertainty_seconds'), 0, 5);
        $rate = self::bounded(config('game-auth.native_runtime_status.requests_per_minute'), 1, 600);
        if ($identities === null || $freshness === null || $uncertainty === null || $rate === null
            || $uncertainty >= $freshness) {
            return null;
        }

        return new self($identities, $freshness, $uncertainty, $rate);
    }

    public function knows(string $identity): bool
    {
        return isset($this->identities[$identity]);
    }

    public function allows(string $identity, string $worldId, string $channelId): bool
    {
        return in_array($worldId.'/'.$channelId, $this->identities[$identity] ?? [], true);
    }

    /**
     * Identity => scope list, refusing (null) any identity of another purpose: native evidence,
     * character bootstrap, premium snapshot and every identity configured under `game-auth.<$otherSection>.identities`,
     * whether or not that purpose is enabled.
     *
     * @return array<string, list<string>>|null
     */
    public static function identities(mixed $raw, string $otherSection): ?array
    {
        $raw = self::decoded($raw);
        if (! is_array($raw) || $raw === [] || array_is_list($raw)) {
            return null;
        }

        $other = self::decoded(config('game-auth.'.$otherSection.'.identities'));
        $otherPurposes = [
            config('game-auth.native_evidence.mtls_client_identity'),
            config('game-auth.character_bootstrap_intent.mtls_client_identity'),
            config('products-entitlements.premium_snapshot.mtls_client_identity'),
            ...(is_array($other) ? array_map('strval', array_keys($other)) : []),
        ];
        $identities = [];
        foreach ($raw as $identity => $scopes) {
            if (! is_string($identity)
                || preg_match(NativeScopeAssignmentReport::IDENTITY, $identity) !== 1
                || in_array($identity, $otherPurposes, true)
                || ! is_array($scopes)
                || $scopes === []
                || ! array_is_list($scopes)) {
                return null;
            }
            $list = [];
            foreach ($scopes as $scope) {
                if (! is_string($scope) || preg_match(self::SCOPE, $scope) !== 1) {
                    return null;
                }
                $list[] = $scope;
            }
            $identities[$identity] = $list;
        }

        return $identities;
    }

    private static function decoded(mixed $raw): mixed
    {
        try {
            return is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return null;
        }
    }

    public static function bounded(mixed $value, int $minimum, int $maximum): ?int
    {
        if (is_string($value) && preg_match('/^(0|[1-9][0-9]{0,3})$/D', $value) === 1) {
            $value = (int) $value;
        }

        return is_int($value) && $value >= $minimum && $value <= $maximum ? $value : null;
    }
}
