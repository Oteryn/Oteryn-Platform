<?php

namespace App\ProductsEntitlements\Store;

use App\CharacterAuthority\CharacterCommandWire;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use JsonException;

/** Wire §11: validate the entire configuration before authenticating a peer. */
final readonly class StoreSettings
{
    /** @param list<string> $enabledUnitKinds */
    private function __construct(
        public string $peerIdentity,
        public string $sourceAuthority,
        public int $requestsPerMinute,
        public int $purchasesPerAccountPerMinute,
        public int $readsPerAccountPerMinute,
        public int $maxPendingOperations,
        public array $enabledUnitKinds,
    ) {}

    public static function current(string $surface): ?self
    {
        $purchase = config('products-entitlements.store.purchase_enabled');
        $delivery = config('products-entitlements.store.delivery_enabled');
        if (! is_bool($purchase) || ! is_bool($delivery) || ($purchase && ! $delivery)
            || ! app()->environment(['testing', 'preproduction'])
            || ! in_array($surface, ['purchase', 'delivery'], true)
            || ($surface === 'purchase' && ! $purchase)
            || ($surface === 'delivery' && ! $delivery && config('character-authority.restore.enabled') !== true)) {
            return null;
        }

        $identity = config('products-entitlements.store.mtls_client_identity');
        $source = config('products-entitlements.store.source_authority');
        $kinds = self::decoded(config('products-entitlements.store.enabled_unit_kinds'), false);
        if (! is_string($identity) || preg_match('/^[\x20-\x7e]{1,128}$/D', $identity) !== 1
            || self::overlaps($identity) || ! is_string($source) || ! CharacterCommandWire::isSourceAuthority($source)
            || ! is_array($kinds) || ! array_is_list($kinds)) {
            return null;
        }
        foreach ($kinds as $kind) {
            if (! is_string($kind) || ! in_array($kind, ['PREMIUM_TIME', 'COSMETIC', 'ITEM'], true)) {
                return null;
            }
        }
        if (count(array_unique($kinds)) !== count($kinds)) {
            return null;
        }

        $bounds = [];
        foreach (['requests_per_minute' => 6000, 'purchases_per_account_per_minute' => 60,
            'reads_per_account_per_minute' => 600, 'max_pending_operations' => 100000] as $key => $maximum) {
            $value = config('products-entitlements.store.'.$key);
            if (is_string($value) && preg_match('/^[1-9][0-9]{0,5}$/D', $value) === 1) {
                $value = (int) $value;
            }
            if (! is_int($value) || $value < 1 || $value > $maximum) {
                return null;
            }
            $bounds[$key] = $value;
        }

        return new self($identity, $source, $bounds['requests_per_minute'], $bounds['purchases_per_account_per_minute'],
            $bounds['reads_per_account_per_minute'], $bounds['max_pending_operations'], $kinds);
    }

    private static function overlaps(string $identity): bool
    {
        $others = [config('game-auth.native_evidence.mtls_client_identity'),
            config('game-auth.character_bootstrap_intent.mtls_client_identity'),
            config('products-entitlements.premium_snapshot.mtls_client_identity'),
            ...NativeAccountCharactersSettings::configuredPublisherIdentities()];
        foreach (['game-auth.native_runtime_status.identities', 'game-auth.native_scope_assignment.identities',
            'character-authority.commands.identities', 'character-authority.restore.identities',
            'game-projections.public.identities'] as $key) {
            $decoded = self::decoded(config($key));
            if (is_array($decoded)) {
                array_push($others, ...(array_is_list($decoded) ? $decoded : array_keys($decoded)));
            }
        }

        return in_array($identity, $others, true);
    }

    private static function decoded(mixed $raw, bool $associative = true): mixed
    {
        try {
            return is_string($raw) ? json_decode($raw, $associative, 4, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return null;
        }
    }
}
