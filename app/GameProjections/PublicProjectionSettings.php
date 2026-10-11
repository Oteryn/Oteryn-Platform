<?php

namespace App\GameProjections;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use JsonException;

/**
 * Testing/preproduction-only consumer configuration for Game Public Projections v1 (§3, §5.1).
 *
 * Off unless `game-projections.public.enabled` is exactly true. The public-projection identities are
 * dedicated: if any of them is also configured for another internal mTLS purpose (LCFA, runtime
 * status, scope assignment, native evidence, bootstrap intent, premium snapshot), the whole
 * consumer is unavailable, so a public-projection certificate is never accepted elsewhere and no
 * other certificate is accepted here.
 */
final readonly class PublicProjectionSettings
{
    /** Platform's clock uncertainty for watermark validation and staleness (§5.1). */
    public const CLOCK_UNCERTAINTY_SECONDS = 1;

    /** Staleness bound S for testing and preproduction (§5.1, U-PP1). */
    public const STALE_SECONDS = 30;

    /** @param list<string> $identities */
    private function __construct(
        private array $identities,
        private string $sourceAuthority,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('game-projections.public.enabled') !== true || ! app()->environment(['testing', 'preproduction'])) {
            return null;
        }

        $identities = self::identities(config('game-projections.public.identities'));
        $sourceAuthority = config('game-projections.public.source_authority');
        $rate = NativeRuntimeStatusSettings::bounded(config('game-projections.public.requests_per_minute', 1200), 1, 6000);
        if ($identities === null
            || ! is_string($sourceAuthority)
            || preg_match(PublicProjectionWire::AUTHORITY, $sourceAuthority) !== 1
            || $rate === null) {
            return null;
        }

        return new self($identities, $sourceAuthority, $rate);
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
        $decoded = self::decode($raw);
        if ($decoded === null || $decoded === [] || ! array_is_list($decoded)) {
            return null;
        }

        $other = NativeAccountCharactersSettings::configuredPublisherIdentities();
        foreach ([
            config('game-auth.native_evidence.mtls_client_identity'),
            config('game-auth.character_bootstrap_intent.mtls_client_identity'),
            config('products-entitlements.premium_snapshot.mtls_client_identity'),
        ] as $single) {
            if (is_string($single)) {
                $other[] = $single;
            }
        }
        foreach (['native_runtime_status', 'native_scope_assignment'] as $section) {
            $map = self::decode(config('game-auth.'.$section.'.identities'));
            if ($map !== null && ! array_is_list($map)) {
                array_push($other, ...array_map('strval', array_keys($map)));
            }
        }

        $identities = [];
        foreach ($decoded as $identity) {
            if (! is_string($identity)
                || preg_match(NativeScopeAssignmentReport::IDENTITY, $identity) !== 1
                || in_array($identity, $other, true)
                || in_array($identity, $identities, true)) {
                return null;
            }
            $identities[] = $identity;
        }

        return $identities;
    }

    /** @return array<array-key, mixed>|null */
    private static function decode(mixed $raw): ?array
    {
        try {
            $decoded = is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return null;
        }

        return is_array($decoded) ? $decoded : null;
    }
}
