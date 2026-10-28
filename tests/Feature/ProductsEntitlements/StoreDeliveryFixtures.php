<?php

namespace Tests\Feature\ProductsEntitlements;

use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use App\Identity\Models\Identity;
use App\ProductsEntitlements\Store\Models\StoreFulfilmentUnit;
use App\ProductsEntitlements\Store\Models\StoreOperation;
use App\ProductsEntitlements\Store\Models\StoreOrder;
use App\ProductsEntitlements\Store\Models\StoreOrderLine;
use App\ProductsEntitlements\Store\Models\StorePurchaseRequest;
use App\ProductsEntitlements\Store\StoreCatalogImporter;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;

trait StoreDeliveryFixtures
{
    private const SOURCE = 'oteryn-game/store-consumer';

    private const PEER = 'CN=store-delivery-tests';

    protected function configureDelivery(): void
    {
        self::assertSame('pgsql', DB::getDriverName());
        config(['products-entitlements.store.purchase_enabled' => false,
            'products-entitlements.store.delivery_enabled' => true,
            'products-entitlements.store.mtls_client_identity' => self::PEER,
            'products-entitlements.store.source_authority' => self::SOURCE]);
    }

    /** @return array<string, mixed> */
    protected function fixture(string $name): array
    {
        $raw = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$name.'.json'));
        self::assertIsString($raw);
        /** @var array<string, mixed> $data */
        $data = json_decode($raw, true, 128, JSON_THROW_ON_ERROR);

        return $data;
    }

    protected function seedDelivery(): void
    {
        $item = $this->fixture('intent.deliver-item');
        $identity = Identity::query()->create(['email' => 'store-delivery@example.test',
            'account_id' => $item['account_id'], 'password' => Hash::make('Store-fixture-password-9!')]);
        $raw = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
        self::assertIsString($raw);
        $revision = app(StoreCatalogImporter::class)->import($raw);
        $request = StorePurchaseRequest::query()->create(['purchase_request_id' => (string) Str::uuid7(),
            'account_id' => $identity->account_id, 'character_id' => (string) Str::uuid7(), 'world_id' => (string) Str::uuid7(),
            'catalog_revision' => $revision, 'offer_key' => 'test-supply', 'currency' => 'COINS', 'unit_price' => 25,
            'quantity' => 1, 'request_bytes' => '{}', 'request_digest' => hash('sha256', '{}'),
            'result' => 'ACCEPTED', 'answer_bytes' => '{}', 'decided_at' => now()]);
        $order = StoreOrder::query()->create(['order_id' => $item['order_id'], 'purchase_request_id' => $request->purchase_request_id,
            'identity_id' => $identity->id, 'wallet_idempotency_key' => 'store-purchase:'.$request->purchase_request_id,
            'currency' => 'COINS', 'total_price' => 25, 'created_at' => now()]);
        $line = StoreOrderLine::query()->create(['line_id' => (string) Str::uuid7(), 'order_id' => $order->order_id,
            'catalog_revision' => $revision, 'offer_key' => 'test-supply', 'quantity' => 1, 'unit_price' => 25]);
        foreach (['premium-time', 'cosmetic', 'item'] as $index => $kind) {
            $intent = $this->fixture('intent.deliver-'.$kind);
            StoreFulfilmentUnit::query()->create([...array_intersect_key($intent, array_flip(['entitlement_id', 'account_id',
                'product_id', 'product_version', 'unit_kind', 'grant_key', 'grant_revision', 'quantity'])),
                'line_id' => $line->line_id, 'unit_index' => $index, 'state' => 'DELIVERY_PENDING', 'created_at' => now()]);
            $this->seedIntent($intent);
        }
        foreach (['claim', 'revoke-unclaimed', 'reclaim', 'flag'] as $kind) {
            $this->seedIntent($this->fixture('intent.'.$kind));
        }
    }

    /** @param array<string, mixed> $intent */
    protected function seedIntent(array $intent): void
    {
        $raw = StoreWire::encode($intent, 4096);
        StoreOperation::query()->create([...array_intersect_key($intent, array_flip(['source_revision', 'operation_id',
            'issuer_authority', 'operation_kind', 'entitlement_id', 'delivery_operation_id', 'reversal_reason'])),
            'intent_bytes' => $raw, 'binding_digest' => hash('sha256', $raw), 'state' => 'PENDING', 'created_at' => now()]);
    }

    /** @param array<string, mixed> $fields
     * @return array<string, mixed>
     */
    protected function pageRequest(bool $reconcile = false, array $fields = []): array
    {
        return array_replace($this->fixture($reconcile ? 'reconcile-request.valid' : 'pending-request.valid'),
            ['after_source_revision' => '0', 'after_operation_id' => '00000000-0000-0000-0000-000000000000'], $fields);
    }

    /** @param array<string, mixed>|string $body
     * @param  array<string, mixed>  $server
     * @return TestResponse<Response>
     */
    protected function deliveryRequest(string $path, array|string $body, array $server = []): TestResponse
    {
        return $this->call('POST', '/internal/v1/products-entitlements/store/operations/'.$path, [], [], [], array_replace([
            'CONTENT_TYPE' => 'application/json', 'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3', 'SSL_CLIENT_S_DN' => self::PEER,
        ], $server), is_string($body) ? $body : StoreWire::encode($body, 8192));
    }

    /** @param TestResponse<Response> $response */
    protected function responseBytes(TestResponse $response): string
    {
        $raw = $response->getContent();
        self::assertIsString($raw);

        return $raw;
    }

    protected function assertWireSchema(string $raw, string $schema): void
    {
        $schemaJson = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$schema.'.schema.json'));
        self::assertIsString($schemaJson);
        self::assertSame([], app(BundledJsonSchemaValidator::class)->validate(json_decode($raw, false, 128, JSON_THROW_ON_ERROR), $schemaJson));
    }
}
