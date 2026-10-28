<?php

namespace App\ProductsEntitlements\Store\Purchase;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire as Wire;
use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use InvalidArgumentException;

/** @phpstan-type Binding array{contract_version:int,operation:string,source_authority:string,purchase_request_id:string,account_id:string,character_id:string,world_id:string,catalog_revision:string,offer_key:string,currency:string,unit_price:string,quantity:int} */
final class StorePurchaseInput
{
    /** @return Binding */
    public function decode(string $raw): array
    {
        $object = Wire::decodeExact($raw, 1024);
        $schema = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/purchase-request.schema.json'));
        if ($schema === false) {
            throw new InvalidArgumentException('Missing Store schema.');
        }
        // Owner Q2 replaces the proposed numeric revision with the canonical import digest.
        $schema = str_replace('^[1-9][0-9]{0,19}$', '^[0-9a-f]{64}$', $schema);
        if ((new BundledJsonSchemaValidator)->validate(json_decode($raw, false, 4, JSON_THROW_ON_ERROR), $schema) !== []
            || array_keys($object) !== ['contract_version', 'operation', 'source_authority', 'purchase_request_id',
                'account_id', 'character_id', 'world_id', 'catalog_revision', 'offer_key', 'currency', 'unit_price', 'quantity']) {
            throw new InvalidArgumentException('Invalid Store purchase body.');
        }
        /** @var Binding $object */
        if (Wire::compareUint64($object['unit_price'], (string) PHP_INT_MAX) > 0) {
            throw new InvalidArgumentException('Store price exceeds its bound.');
        }

        return $object;
    }
}
