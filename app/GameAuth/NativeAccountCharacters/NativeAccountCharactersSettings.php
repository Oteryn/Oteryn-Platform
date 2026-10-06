<?php

namespace App\GameAuth\NativeAccountCharacters;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use JsonException;

/**
 * Testing/preproduction-only LCFA push consumer configuration. Multiple Character Authority hosts
 * may hold dedicated projection certificates, but every configured identity shares one expected
 * Character Authority namespace. Any identity reused by another internal purpose makes the whole
 * consumer unavailable.
 */
final readonly class NativeAccountCharactersSettings
{
    /** @param list<string> $identities */
    private function __construct(
        private array $identities,
        private string $sourceAuthority,
        public int $freshnessSeconds,
        public int $clockUncertaintySeconds,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('game-auth.native_account_characters.enabled') !== true || app()->environment('production')) {
            return null;
        }

        $identities = self::identities(config('game-auth.native_account_characters.identities'));
        $sourceAuthority = self::sourceAuthority(config('game-auth.native_account_characters.source_authority'));
        $freshness = NativeRuntimeStatusSettings::bounded(config('game-auth.native_account_characters.freshness_seconds'), 1, 120);
        $uncertainty = NativeRuntimeStatusSettings::bounded(config('game-auth.native_account_characters.clock_uncertainty_seconds'), 0, 5);
        $rate = NativeRuntimeStatusSettings::bounded(config('game-auth.native_account_characters.requests_per_minute'), 1, 600);
        if ($identities === null || $sourceAuthority === null || $freshness === null || $uncertainty === null || $rate === null || $uncertainty >= $freshness) {
            return null;
        }

        return new self($identities, $sourceAuthority, $freshness, $uncertainty, $rate);
    }

    /** @return list<string> Configured publisher subjects, even while the LCFA switch is off. */
    public static function configuredPublisherIdentities(): array
    {
        $raw = config('game-auth.native_account_characters.identities');
        try {
            $decoded = is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return [];
        }
        if (! is_array($decoded) || ! array_is_list($decoded)) {
            return [];
        }

        return array_values(array_filter($decoded, 'is_string'));
    }

    public function knows(string $identity): bool
    {
        return in_array($identity, $this->identities, true);
    }

    public function allows(string $identity, string $sourceAuthority): bool
    {
        return $this->knows($identity) && hash_equals($this->sourceAuthority, $sourceAuthority);
    }

    /** @return list<string>|null */
    private static function identities(mixed $raw): ?array
    {
        try {
            $decoded = is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return null;
        }
        if (! is_array($decoded) || $decoded === [] || ! array_is_list($decoded)) {
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
            if (is_array($other) && ! array_is_list($other)) {
                array_push($otherIdentities, ...array_map('strval', array_keys($other)));
            }
        }

        $identities = [];
        foreach ($decoded as $identity) {
            if (! is_string($identity)
                || preg_match(NativeScopeAssignmentReport::IDENTITY, $identity) !== 1
                || in_array($identity, $otherIdentities, true)
                || in_array($identity, $identities, true)) {
                return null;
            }
            $identities[] = $identity;
        }

        return $identities;
    }

    private static function sourceAuthority(mixed $raw): ?string
    {
        if (! is_string($raw) || preg_match('/^[A-Za-z0-9._:\\/-]{1,128}$/D', $raw) !== 1) {
            return null;
        }

        return $raw;
    }
}
