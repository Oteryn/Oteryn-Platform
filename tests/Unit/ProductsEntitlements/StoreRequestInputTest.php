<?php

namespace Tests\Unit\ProductsEntitlements;

use App\ProductsEntitlements\Store\Purchase\StoreAccountLimiter;
use App\ProductsEntitlements\Store\Purchase\StorePurchaseInput;
use App\ProductsEntitlements\Store\Purchase\StoreReadInput;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Cache\RateLimiter;
use InvalidArgumentException;
use Tests\TestCase;

/** Driver-independent Store request parsing; runs on every test lane. */
final class StoreRequestInputTest extends TestCase
{
    private const ACCOUNT = '0192b6a0-4c00-7000-8000-000000000101';

    public function test_purchase_input_accepts_the_digest_revision_and_preserves_the_binding(): void
    {
        $decoded = (new StorePurchaseInput)->decode($this->purchase());

        self::assertSame(str_repeat('a', 64), $decoded['catalog_revision']);
        self::assertSame('premium-30d', $decoded['offer_key']);
        self::assertSame('250', $decoded['unit_price']);
        self::assertSame(1, $decoded['quantity']);
    }

    public function test_purchase_input_rejects_the_numeric_proposed_revision(): void
    {
        $this->expectException(InvalidArgumentException::class);
        (new StorePurchaseInput)->decode($this->purchase(['catalog_revision' => '37']));
    }

    public function test_purchase_input_rejects_a_price_above_the_signed_bound(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('Store price exceeds its bound.');
        (new StorePurchaseInput)->decode($this->purchase(['unit_price' => '9223372036854775808']));
    }

    public function test_purchase_input_rejects_non_canonical_bytes(): void
    {
        $this->expectException(InvalidArgumentException::class);
        (new StorePurchaseInput)->decode($this->purchase()."\n");
    }

    public function test_balance_input_accepts_the_closed_body(): void
    {
        $decoded = (new StoreReadInput)->decode($this->read('ReadStoreBalanceV1'), false);

        self::assertSame(self::ACCOUNT, $decoded['account_id']);
    }

    public function test_balance_input_rejects_history_members(): void
    {
        $this->expectException(InvalidArgumentException::class);
        (new StoreReadInput)->decode($this->read('ReadStoreBalanceV1', ['before_entry_id' => '0', 'max_entries' => 2]), false);
    }

    public function test_history_input_accepts_a_first_page_and_a_cursor(): void
    {
        $input = new StoreReadInput;

        $first = $input->decode($this->read('ReadStoreHistoryPageV1', ['before_entry_id' => '0', 'max_entries' => 2]), true);
        $next = $input->decode($this->read('ReadStoreHistoryPageV1', ['before_entry_id' => '41', 'max_entries' => 2]), true);

        self::assertSame('0', $first['before_entry_id']);
        self::assertSame('41', $next['before_entry_id']);
        self::assertSame(2, $next['max_entries']);
    }

    public function test_history_input_rejects_missing_paging_members(): void
    {
        $this->expectException(InvalidArgumentException::class);
        (new StoreReadInput)->decode($this->read('ReadStoreHistoryPageV1'), true);
    }

    public function test_account_limiter_refuses_after_the_per_account_limit(): void
    {
        $limiter = new StoreAccountLimiter($this->app->make(RateLimiter::class));

        self::assertNull($limiter->refuse(self::ACCOUNT, 1, 'purchase'));
        $refused = $limiter->refuse(self::ACCOUNT, 1, 'purchase');

        self::assertNotNull($refused);
        self::assertSame(429, $refused->getStatusCode());
        self::assertGreaterThanOrEqual(1, (int) $refused->headers->get('Retry-After'));
        self::assertNull($limiter->refuse(self::ACCOUNT, 1, 'balance'));
    }

    /** @param array<string, mixed> $changes */
    private function purchase(array $changes = []): string
    {
        return StoreWire::encode(array_replace([
            'contract_version' => 1,
            'operation' => 'PurchaseStoreOfferV1',
            'source_authority' => 'oteryn-game/store-consumer',
            'purchase_request_id' => '0192b6a0-4c00-7000-8000-000000000401',
            'account_id' => self::ACCOUNT,
            'character_id' => '0192b6a0-4c00-7000-8000-000000000201',
            'world_id' => '0192b6a0-4c00-7000-8000-000000000301',
            'catalog_revision' => str_repeat('a', 64),
            'offer_key' => 'premium-30d',
            'currency' => 'COINS',
            'unit_price' => '250',
            'quantity' => 1,
        ], $changes), 1024);
    }

    /** @param array<string, mixed> $extra */
    private function read(string $operation, array $extra = []): string
    {
        return StoreWire::encode(array_merge([
            'contract_version' => 1,
            'operation' => $operation,
            'source_authority' => 'oteryn-game/store-consumer',
            'account_id' => self::ACCOUNT,
        ], $extra), 512);
    }
}
