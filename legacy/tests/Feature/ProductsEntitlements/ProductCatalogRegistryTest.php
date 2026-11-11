<?php

namespace Tests\Feature\ProductsEntitlements;

use App\ProductsEntitlements\Catalog\ProductCatalogContract;
use App\ProductsEntitlements\Catalog\ProductCatalogException;
use App\ProductsEntitlements\Catalog\ProductCatalogRegistry;
use App\ProductsEntitlements\Catalog\ProductCatalogVersion;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class ProductCatalogRegistryTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        config()->set('payments.allowed_currencies', ['PLN', 'EUR']);
        config()->set('payments.maximum_order_amount_minor', 100_000_000);
    }

    public function test_registers_one_immutable_localized_product_version(): void
    {
        $version = $this->registry()->register(
            'oteryn.premium_time',
            1,
            ProductCatalogContract::PROFILE_GAME_ACCOUNT,
            ProductCatalogContract::SCOPE_ACCOUNT,
            'PLN',
            3_999,
            $this->presentations(),
            1_800_000_000,
            1_800_086_400,
        );

        self::assertSame('oteryn.premium_time', $version->productId);
        self::assertSame(1, $version->version);
        self::assertSame('B', $version->deliveryProfile);
        self::assertSame('account', $version->targetScope);
        self::assertSame('PLN', $version->currency);
        self::assertSame(3_999, $version->priceMinor);
        self::assertSame(1_800_000_000, $version->availableFrom);
        self::assertSame(1_800_086_400, $version->availableUntil);
        self::assertMatchesRegularExpression('/\A[0-9a-f]{64}\z/', $version->payloadSha256);
        self::assertSame($this->presentations(), $version->presentations);
        self::assertSame(1, DB::table('product_catalog_products')->count());
        self::assertSame(1, DB::table('product_catalog_versions')->count());
        self::assertSame(2, DB::table('product_catalog_presentations')->count());
    }

    public function test_exact_version_retry_is_idempotent_but_changed_reuse_conflicts(): void
    {
        $registry = $this->registry();
        $first = $registry->register(
            'oteryn.coin_pack.small',
            7,
            ProductCatalogContract::PROFILE_WALLET_PACKAGE,
            ProductCatalogContract::SCOPE_WALLET,
            'EUR',
            999,
            $this->presentations('Small coin package', 'Mały pakiet monet'),
        );
        $retry = $registry->register(
            'oteryn.coin_pack.small',
            7,
            ProductCatalogContract::PROFILE_WALLET_PACKAGE,
            ProductCatalogContract::SCOPE_WALLET,
            'EUR',
            999,
            $this->presentations('Small coin package', 'Mały pakiet monet'),
        );

        self::assertSame($first->payloadSha256, $retry->payloadSha256);
        self::assertSame(1, DB::table('product_catalog_versions')->count());

        try {
            $registry->register(
                'oteryn.coin_pack.small',
                7,
                ProductCatalogContract::PROFILE_WALLET_PACKAGE,
                ProductCatalogContract::SCOPE_WALLET,
                'EUR',
                1_099,
                $this->presentations('Small coin package', 'Mały pakiet monet'),
            );
            self::fail('Changed reuse of one product version must fail.');
        } catch (ProductCatalogException $exception) {
            self::assertSame('version_conflict', $exception->reason);
        }

        self::assertSame(999, $registry->find('oteryn.coin_pack.small', 7)?->priceMinor);
    }

    public function test_new_version_does_not_reinterpret_previous_version(): void
    {
        $registry = $this->registry();
        $registry->register(
            'oteryn.account_tier',
            1,
            ProductCatalogContract::PROFILE_PLATFORM_ONLY,
            ProductCatalogContract::SCOPE_ACCOUNT,
            'PLN',
            1_000,
            $this->presentations('Account tier', 'Poziom konta'),
        );
        $registry->register(
            'oteryn.account_tier',
            2,
            ProductCatalogContract::PROFILE_PLATFORM_ONLY,
            ProductCatalogContract::SCOPE_ACCOUNT,
            'PLN',
            1_500,
            $this->presentations('Account tier plus', 'Poziom konta plus'),
        );

        $first = $registry->find('oteryn.account_tier', 1);
        $second = $registry->find('oteryn.account_tier', 2);
        self::assertInstanceOf(ProductCatalogVersion::class, $first);
        self::assertInstanceOf(ProductCatalogVersion::class, $second);
        self::assertSame(1_000, $first->priceMinor);
        self::assertSame('Account tier', $first->presentations['en']['name']);
        self::assertSame(1_500, $second->priceMinor);
        self::assertSame(2, DB::table('product_catalog_versions')->count());
    }

    public function test_rejects_unsupported_currency_invalid_window_and_missing_locale(): void
    {
        /** @var list<array{string,string,int,array<string, array{name:string,description:string}>,int|null,int|null}> $cases */
        $cases = [
            ['currency_unsupported', 'USD', 1_000, $this->presentations(), null, null],
            ['availability_invalid', 'PLN', 1_000, $this->presentations(), 200, 100],
            ['presentation_invalid', 'PLN', 1_000, ['en' => ['name' => 'Only English', 'description' => 'Missing Polish']], null, null],
        ];

        foreach ($cases as [$reason, $currency, $price, $presentations, $from, $until]) {
            try {
                $this->registry()->register(
                    'oteryn.test_product',
                    1,
                    ProductCatalogContract::PROFILE_PLATFORM_ONLY,
                    ProductCatalogContract::SCOPE_PLATFORM,
                    $currency,
                    $price,
                    $presentations,
                    $from,
                    $until,
                );
                self::fail("Expected {$reason}.");
            } catch (ProductCatalogException $exception) {
                self::assertSame($reason, $exception->reason);
            }
        }

        self::assertSame(0, DB::table('product_catalog_versions')->count());
    }

    private function registry(): ProductCatalogRegistry
    {
        return $this->app->make(ProductCatalogRegistry::class);
    }

    /**
     * @return array<string, array{name:string,description:string}>
     */
    private function presentations(
        string $english = 'Premium time',
        string $polish = 'Czas premium',
    ): array {
        return [
            'en' => ['name' => $english, 'description' => 'Synthetic inactive catalogue description.'],
            'pl' => ['name' => $polish, 'description' => 'Syntetyczny opis nieaktywnego katalogu.'],
        ];
    }
}
