<?php

namespace App\ProductsEntitlements\Store\Delivery;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire as Wire;
use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use App\ProductsEntitlements\Store\StoreWire;
use InvalidArgumentException;

/**
 * @phpstan-type Page array{after_source_revision:string,after_operation_id:string,max_entries:int}
 * @phpstan-type Key array{issuer_authority:string,operation_kind:string,operation_id:string}
 * @phpstan-type Receipt array{issuer_authority:string,operation_kind:string,operation_id:string,binding_digest:string,outcome:string,result_code:string,account_id:string,entitlement_id:string,product_id:string,product_version:string,recovery_generation:string,decided_at:string,delivered_quantity?:int,character_id?:string,world_id?:string,minted_quantity?:int,minted_units_digest?:string,revoked_quantity?:int,reclaimed_quantity?:int,shortfall_quantity?:int}
 */
final class StoreDeliveryRequest
{
    private const HEAD = ['contract_version', 'operation', 'source_authority'];

    public function __construct(private readonly BundledJsonSchemaValidator $schema) {}

    /** @return Page */
    public function page(string $raw, bool $reconcile): array
    {
        $data = $this->decode($raw, $reconcile ? 'reconcile-request' : 'pending-request', 512);
        $this->members($data, [...self::HEAD, 'after_source_revision', 'after_operation_id', 'max_entries']);
        /** @var Page $data Schema proves the types; the numeric ceiling and cursor pairing follow. */
        $after = $data['after_source_revision'];
        if ($after !== '0') {
            Wire::uint64($data, 'after_source_revision');
        }
        if (($after === '0') !== ($data['after_operation_id'] === '00000000-0000-0000-0000-000000000000')) {
            throw new InvalidArgumentException('Invalid Store cursor pair.');
        }

        return $data;
    }

    /** @return Key */
    public function key(string $raw): array
    {
        $data = $this->decode($raw, 'read-request', 512);
        $this->members($data, [...self::HEAD, 'issuer_authority', 'operation_kind', 'operation_id']);

        /** @var Key $data Schema proves the key. */
        return $data;
    }

    /** @return Receipt */
    public function receipt(string $raw): array
    {
        $data = $this->decode($raw, 'receipt', 1024);
        /** @var Receipt $data Schema proves the closed kind, outcome, code and field types. */
        $committed = $data['outcome'] === 'COMMITTED' ? match ($data['operation_kind']) {
            'DELIVER' => ['delivered_quantity'],
            'CLAIM' => ['character_id', 'world_id', 'minted_quantity', 'minted_units_digest'],
            'REVOKE_UNCLAIMED' => ['revoked_quantity'],
            'RECLAIM' => ['character_id', 'world_id', 'reclaimed_quantity', 'shortfall_quantity'],
            default => [],
        } : [];
        $this->members($data, [...self::HEAD, 'issuer_authority', 'operation_kind', 'operation_id', 'binding_digest',
            'outcome', 'result_code', 'account_id', 'entitlement_id', 'product_id', 'product_version',
            ...$committed, 'recovery_generation', 'decided_at']);
        foreach (['product_version', 'recovery_generation', 'decided_at'] as $field) {
            Wire::uint64($data, $field);
        }

        return $data;
    }

    /** @param array<string, mixed> $data
     * @param  list<string>  $members
     */
    private function members(array $data, array $members): void
    {
        if (array_keys($data) !== $members) {
            throw new InvalidArgumentException('Invalid Store member order.');
        }
    }

    /** @return array<string, mixed> */
    private function decode(string $raw, string $name, int $bound): array
    {
        $data = Wire::decodeExact($raw, $bound);
        if (StoreWire::encode($data, $bound) !== $raw) {
            throw new InvalidArgumentException('Invalid Store canonical bytes.');
        }
        $schema = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$name.'.schema.json'));
        if ($schema === false) {
            throw new StoreDeliveryRefused(503);
        }
        if ($this->schema->validate(json_decode($raw, false, 4, JSON_THROW_ON_ERROR), $schema) !== []) {
            throw new InvalidArgumentException('Invalid Store message.');
        }

        return $data;
    }
}
