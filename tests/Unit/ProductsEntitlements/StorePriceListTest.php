<?php

namespace Tests\Unit\ProductsEntitlements;

use App\ProductsEntitlements\Store\StorePriceList;
use InvalidArgumentException;
use Tests\TestCase;

/** Driver-independent Game price list validation; runs on every test lane. */
final class StorePriceListTest extends TestCase
{
    private const FIXTURES = 'docs/contracts/fixtures/store-price-list-v1/';

    public function test_valid_price_list_is_returned_with_its_offers_and_units(): void
    {
        $data = $this->priceList()->validate($this->fixture('price-list.valid.json'));

        self::assertSame(1, $data['contract_version']);
        self::assertSame('test-supply', $data['offers'][0]['offer_key']);
        self::assertSame('25', $data['offers'][0]['unit_price']);
        self::assertSame('ITEM', $data['offers'][0]['units'][0]['unit_kind'] ?? null);
    }

    public function test_every_invalid_fixture_is_rejected(): void
    {
        $files = glob(base_path(self::FIXTURES.'price-list.invalid-*.json'));
        self::assertIsArray($files);
        self::assertCount(5, $files);
        $rejected = 0;
        foreach ($files as $path) {
            $raw = file_get_contents($path);
            self::assertIsString($raw);
            try {
                $this->priceList()->validate($raw);
            } catch (InvalidArgumentException) {
                $rejected++;
            }
        }

        self::assertSame(5, $rejected);
    }

    public function test_non_canonical_reordered_and_malformed_inputs_are_rejected(): void
    {
        $raw = $this->fixture('price-list.valid.json');
        $rejected = 0;
        $invalid = [
            '',
            '{',
            $raw."\n",
            str_replace('"contract_version":1,"offers":', '"offers":', $raw),
            str_replace('"offer_key":"test-supply","currency":"COINS"', '"currency":"COINS","offer_key":"test-supply"', $raw),
        ];
        foreach ($invalid as $candidate) {
            try {
                $this->priceList()->validate($candidate);
            } catch (InvalidArgumentException) {
                $rejected++;
            }
        }

        self::assertSame(count($invalid), $rejected);
    }

    private function priceList(): StorePriceList
    {
        return $this->app->make(StorePriceList::class);
    }

    private function fixture(string $name): string
    {
        $raw = file_get_contents(base_path(self::FIXTURES.$name));
        self::assertIsString($raw);

        return $raw;
    }
}
