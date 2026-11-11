<?php

namespace App\ProductsEntitlements\Store\Purchase;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire as Wire;
use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use InvalidArgumentException;

final class StoreReadInput
{
    /** @return array{account_id:string,before_entry_id?:string,max_entries?:int} */
    public function decode(string $raw, bool $history): array
    {
        $object = Wire::decodeExact($raw, 512);
        $name = $history ? 'history' : 'balance';
        $schema = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$name.'-request.schema.json'));
        $keys = ['contract_version', 'operation', 'source_authority', 'account_id'];
        if ($history) {
            array_push($keys, 'before_entry_id', 'max_entries');
        }
        if ($schema === false || array_keys($object) !== $keys
            || (new BundledJsonSchemaValidator)->validate(json_decode($raw, false, 4, JSON_THROW_ON_ERROR), $schema) !== []) {
            throw new InvalidArgumentException('Invalid Store read body.');
        }
        /** @var array{account_id:string,before_entry_id?:string,max_entries?:int} $object */
        if ($history && ($object['before_entry_id'] ?? '0') !== '0') {
            Wire::uint64($object, 'before_entry_id');
        }

        return $object;
    }
}
