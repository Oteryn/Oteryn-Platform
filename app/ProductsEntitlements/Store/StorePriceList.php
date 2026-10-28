<?php

namespace App\ProductsEntitlements\Store;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use InvalidArgumentException;
use JsonException;
use stdClass;

/**
 * S-5 Game input: closed schema plus canonical bytes, bounded integers and unique offer keys.
 *
 * @phpstan-type Unit array{product_id:string,product_version:string,unit_kind:string,grant_key:string,grant_revision:string,quantity:int}
 * @phpstan-type Offer array{offer_key:string,currency:string,unit_price:string,max_quantity:int,purchasable:bool,units:list<Unit>}
 * @phpstan-type PriceList array{contract_version:1,offers:non-empty-list<Offer>}
 */
final class StorePriceList
{
    public const MAX_BYTES = 16777216;

    public function __construct(private readonly BundledJsonSchemaValidator $schema) {}

    /** @return PriceList */
    public function validate(string $raw): array
    {
        if ($raw === '' || strlen($raw) > self::MAX_BYTES) {
            throw new InvalidArgumentException('Store price list size is invalid.');
        }
        try {
            $document = json_decode($raw, false, 8, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Store price list JSON is invalid.', previous: $exception);
        }
        $schema = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.schema.json'));
        if ($schema === false || $this->schema->validate($document, $schema) !== []) {
            throw new InvalidArgumentException('Store price list does not match the closed schema.');
        }
        $this->assertOrder($document);
        /** @var PriceList $data Schema validation above proves this structure. */
        $data = json_decode($raw, true, 8, JSON_THROW_ON_ERROR);
        if (StoreWire::encode($data, self::MAX_BYTES) !== $raw) {
            throw new InvalidArgumentException('Store price list must use canonical bytes and member order.');
        }
        $seen = [];
        foreach ($data['offers'] as $offer) {
            if (NativeAccountCharactersWire::compareUint64($offer['unit_price'], '9223372036854775807') > 0
                || isset($seen[$offer['offer_key']])) {
                throw new InvalidArgumentException('Store price list offer order, price or uniqueness is invalid.');
            }
            $seen[$offer['offer_key']] = true;
            foreach ($offer['units'] as $unit) {
                NativeAccountCharactersWire::uint64($unit, 'product_version');
                NativeAccountCharactersWire::uint64($unit, 'grant_revision');
            }
        }

        return $data;
    }

    /** Schema proves the shapes; inspect the original objects to enforce member order as well. */
    private function assertOrder(mixed $value): void
    {
        if ($value instanceof stdClass) {
            $members = get_object_vars($value);
            $keys = isset($members['offers']) ? ['contract_version', 'offers']
                : (isset($members['offer_key']) ? ['offer_key', 'currency', 'unit_price', 'max_quantity', 'purchasable', 'units']
                    : ['product_id', 'product_version', 'unit_kind', 'grant_key', 'grant_revision', 'quantity']);
            if (array_keys($members) !== $keys) {
                throw new InvalidArgumentException('Store price list member order is invalid.');
            }
            foreach ($members as $member) {
                $this->assertOrder($member);
            }
        } elseif (is_array($value)) {
            foreach ($value as $member) {
                $this->assertOrder($member);
            }
        }
    }
}
