<?php

namespace App\ProductsEntitlements\Loyalty;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use JsonException;

/** Fail-closed activation and dedicated-peer configuration for the loyalty read. */
final readonly class LoyaltySnapshotSettings
{
    private function __construct(
        public string $peerIdentity,
        public string $producerRevision,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('products-entitlements.loyalty_snapshot.enabled') !== true) {
            return null;
        }
        $identity = self::identity();
        $revision = config('products-entitlements.loyalty_snapshot.producer_revision');
        $rate = NativeRuntimeStatusSettings::bounded(
            config('products-entitlements.loyalty_snapshot.requests_per_minute'),
            1,
            6000,
        );
        if ($identity === null
            || ! is_string($revision)
            || preg_match('/^[0-9a-f]{40}$/D', $revision) !== 1
            || $rate === null) {
            return null;
        }

        return new self($identity, $revision, $rate);
    }

    /** The configured identity, or null when malformed or shared with another internal purpose. */
    public static function identity(): ?string
    {
        $identity = config('products-entitlements.loyalty_snapshot.mtls_client_identity');
        if (! is_string($identity)
            || $identity === ''
            || strlen($identity) > 128
            || preg_match('/^[\x20-\x7e]+$/D', $identity) !== 1
            || in_array($identity, self::otherPurposeIdentities(), true)) {
            return null;
        }

        return $identity;
    }

    /** @return list<string> */
    private static function otherPurposeIdentities(): array
    {
        $identities = [
            config('products-entitlements.premium_snapshot.mtls_client_identity'),
            config('game-auth.native_evidence.mtls_client_identity'),
            config('game-auth.character_bootstrap_intent.mtls_client_identity'),
        ];
        array_push($identities, ...NativeAccountCharactersSettings::configuredPublisherIdentities());
        foreach (['native_runtime_status', 'native_scope_assignment'] as $section) {
            $raw = config('game-auth.'.$section.'.identities');
            try {
                $decoded = is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
            } catch (JsonException) {
                $decoded = null;
            }
            if (is_array($decoded)) {
                array_push($identities, ...array_map('strval', array_keys($decoded)));
            }
        }

        return array_values(array_filter($identities, 'is_string'));
    }
}
