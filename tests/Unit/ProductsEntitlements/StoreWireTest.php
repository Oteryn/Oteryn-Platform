<?php

namespace Tests\Unit\ProductsEntitlements;

use App\ProductsEntitlements\Store\StoreWire;
use InvalidArgumentException;
use PHPUnit\Framework\TestCase;

final class StoreWireTest extends TestCase
{
    public function test_store_canonical_encoding_and_decoding_preserve_bytes_and_source(): void
    {
        $raw = StoreWire::encode(['contract_version' => 1, 'operation' => 'ReadStoreBalanceV1', 'source_authority' => 'game/store:1'], 512);
        self::assertSame('{"contract_version":1,"operation":"ReadStoreBalanceV1","source_authority":"game/store:1"}', $raw);
        self::assertSame('game/store:1', StoreWire::sourceAuthority($raw, 512, 'ReadStoreBalanceV1'));
    }

    public function test_store_encoding_rejects_strings_requiring_escapes(): void
    {
        $this->expectException(InvalidArgumentException::class);
        StoreWire::encode(['secret' => "not\nwire"], 512);
    }

    public function test_store_encoding_rejects_oversized_output(): void
    {
        $this->expectException(InvalidArgumentException::class);
        StoreWire::encode(['value' => str_repeat('x', 512)], 512);
    }
}
