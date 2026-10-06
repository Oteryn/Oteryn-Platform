<?php

namespace App\ProductsEntitlements\Catalog;

final readonly class ProductCatalogVersion
{
    /**
     * @param array<string, array{name:string,description:string}> $presentations
     */
    public function __construct(
        public string $productId,
        public int $version,
        public string $deliveryProfile,
        public string $targetScope,
        public string $currency,
        public int $priceMinor,
        public ?int $availableFrom,
        public ?int $availableUntil,
        public string $payloadSha256,
        public array $presentations,
    ) {}
}
