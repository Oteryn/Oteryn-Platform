<?php

namespace Tests\Feature\ProductsEntitlements;

use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use App\Identity\Models\Identity;
use App\ProductsEntitlements\Store\StoreCatalogActivator;
use App\ProductsEntitlements\Store\StoreCatalogImporter;
use App\ProductsEntitlements\Store\StoreWire;
use App\Wallet\WalletMutator;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

abstract class StorePurchaseTestCase extends TestCase
{
    protected Identity $buyer;

    protected string $revision;

    protected function setUp(): void
    {
        parent::setUp();
        self::assertSame('pgsql', DB::getDriverName());
        config(['products-entitlements.store.purchase_enabled' => true,
            'products-entitlements.store.delivery_enabled' => true,
            'products-entitlements.store.mtls_client_identity' => 'CN=purchase-test',
            'products-entitlements.store.source_authority' => 'game.store.test',
            'products-entitlements.store.enabled_unit_kinds' => ['ITEM', 'COSMETIC', 'PREMIUM_TIME']]);
        $this->buyer = Identity::query()->create(['email' => 'store@example.test', 'password' => 'unused-test-password']);
        $this->revision = $this->catalog();
    }

    protected function catalog(?string $raw = null): string
    {
        if ($raw === null) {
            $raw = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
            self::assertIsString($raw);
        }
        $revision = app(StoreCatalogImporter::class)->import($raw);
        app(StoreCatalogActivator::class)->activate($revision);

        return $revision;
    }

    protected function fund(int $amount = 100, ?Identity $buyer = null): void
    {
        DB::transaction(function () use ($buyer, $amount): void {
            $wallets = app(WalletMutator::class);
            $wallets->applyLocked($wallets->lock(($buyer ?? $this->buyer)->id), 'administrator_adjustment', $amount, 0, 'test:'.Str::uuid7());
        });
    }

    /** @param array<string,mixed> $changes */
    protected function purchaseBody(array $changes = []): string
    {
        return StoreWire::encode(array_replace(['contract_version' => 1, 'operation' => 'PurchaseStoreOfferV1',
            'source_authority' => 'game.store.test', 'purchase_request_id' => (string) Str::uuid7(),
            'account_id' => $this->buyer->account_id, 'character_id' => (string) Str::uuid7(), 'world_id' => (string) Str::uuid7(),
            'catalog_revision' => $this->revision, 'offer_key' => 'test-supply', 'currency' => 'COINS', 'unit_price' => '25', 'quantity' => 1], $changes), 1024);
    }

    /** @param array<string,mixed> $changes */
    protected function readBody(bool $history = false, array $changes = []): string
    {
        $body = ['contract_version' => 1, 'operation' => $history ? 'ReadStoreHistoryPageV1' : 'ReadStoreBalanceV1',
            'source_authority' => 'game.store.test', 'account_id' => $this->buyer->account_id];
        if ($history) {
            $body += ['before_entry_id' => '0', 'max_entries' => 2];
        }

        return StoreWire::encode(array_replace($body, $changes), 512);
    }

    /** @return TestResponse<Response> */
    protected function send(string $path, string $raw): TestResponse
    {
        return $this->call('POST', '/internal/v1/products-entitlements/store/'.$path, [], [], [], [
            'CONTENT_TYPE' => 'application/json', 'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3', 'SSL_CLIENT_S_DN' => 'CN=purchase-test',
        ], $raw);
    }

    protected function schema(string $name, string $raw, bool $q2 = false): void
    {
        $schema = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$name.'.schema.json'));
        self::assertIsString($schema);
        if ($q2) {
            // Project only the delegated Q2 field into the proposed fixture domain.
            $raw = preg_replace('/"catalog_revision":"[0-9a-f]{64}"/', '"catalog_revision":"1"', $raw);
            self::assertIsString($raw);
        }
        self::assertSame([], (new BundledJsonSchemaValidator)->validate(json_decode($raw, false, 8, JSON_THROW_ON_ERROR), $schema));
    }
}
