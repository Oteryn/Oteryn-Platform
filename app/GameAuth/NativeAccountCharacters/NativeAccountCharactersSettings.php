<?php

namespace App\GameAuth\NativeAccountCharacters;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use JsonException;

/**
 * Testing/preproduction-only LCFA push consumer configuration. Publishers are bound as
 * certificate-subject => Character Authority namespace; any identity shared with another internal
 * purpose makes the whole consumer unavailable.
 */
final readonly class NativeAccountCharactersSettings
{
    /** @param array<string, string> $publishers */
    private function __construct(
        private array $publishers,
        public int $freshnessSeconds,
        public int $clockUncertaintySeconds,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('game-auth.native_account_characters.enabled') !== true || app()->environment('production')) {
            return null;
        }

        $publishers = self::publishers(config('game-auth.native_account_characters.publishers'));
        $freshness = NativeRuntimeStatusSettings::bounded(config('game-auth.native_account_characters.freshness_seconds'), 1, 120);
        $uncertainty = NativeRuntimeStatusSettings::bounded(config('game-auth.native_account_characters.clock_uncertainty_seconds'), 0, 5);
        $rate = NativeRuntimeStatusSettings::bounded(config('game-auth.native_account_characters.requests_per_minute'), 1, 600);
        if ($publishers === null || $freshness === null || $uncertainty === null || $rate === null || $uncertainty >= $freshness) {
            return null;
        }

        return new self($publishers, $freshness, $uncertainty, $rate);
    }

    public function knows(string $identity): bool
    {
        return isset($this->publishers[$identity]);
    }

    public function allows(string $identity, string $sourceAuthority): bool
    {
        $authority = $this->publishers[$identity] ?? null;

        return is_string($authority) && hash_equals($authority, $sourceAuthority);
    }

    /** @return array<string, string>|null */
    private static function publishers(mixed $raw): ?array
    {
        try {
            $decoded = is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return null;
        }
        if (! is_array($decoded) || $decoded === [] || array_is_list($decoded)) {
            return null;
        }

        $otherIdentities = [
            config('game-auth.native_evidence.mtls_client_identity'),
            config('game-auth.character_bootstrap_intent.mtls_client_identity'),
            config('products-entitlements.premium_snapshot.mtls_client_identity'),
        ];
        foreach (['native_runtime_status', 'native_scope_assignment'] as $section) {
            $other = config('game-auth.'.$section.'.identities');
            try {
                $other = is_string($other) ? json_decode($other, true, 3, JSON_THROW_ON_ERROR) : $other;
            } catch (JsonException) {
                $other = null;
            }
            if (is_array($other)) {
                array_push($otherIdentities, ...array_map('strval', array_keys($other)));
            }
        }

        $publishers = [];
        foreach ($decoded as $identity => $authority) {
            if (! is_string($identity)
                || preg_match(NativeScopeAssignmentReport::IDENTITY, $identity) !== 1
                || in_array($identity, $otherIdentities, true)
                || ! is_string($authority)
                || preg_match('/^[A-Za-z0-9._:\/-]{1,128}$/D', $authority) !== 1) {
                return null;
            }
            $publishers[$identity] = $authority;
        }

        return $publishers;
    }
}
