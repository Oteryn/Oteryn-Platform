<?php

namespace App\ProductsEntitlements\Catalog;

use Illuminate\Support\Facades\DB;
use JsonException;
use UnexpectedValueException;

final class ProductCatalogRegistry
{
    /**
     * Registers one immutable catalogue version. Exact semantic retries are idempotent; changed
     * reuse of one (product_id, version) is a conflict. Registration does not publish or activate
     * a product and does not authorize payment or entitlement delivery.
     *
     * @param  array<string, array{name:string,description:string}>  $presentations
     */
    public function register(
        string $productId,
        int $version,
        string $deliveryProfile,
        string $targetScope,
        string $currency,
        int $priceMinor,
        array $presentations,
        ?int $availableFrom = null,
        ?int $availableUntil = null,
    ): ProductCatalogVersion {
        $canonicalPresentations = $this->validate(
            $productId,
            $version,
            $deliveryProfile,
            $targetScope,
            $currency,
            $priceMinor,
            $presentations,
            $availableFrom,
            $availableUntil,
        );

        $payload = [
            'product_id' => $productId,
            'version' => $version,
            'delivery_profile' => $deliveryProfile,
            'target_scope' => $targetScope,
            'currency' => $currency,
            'price_minor' => $priceMinor,
            'available_from' => $availableFrom,
            'available_until' => $availableUntil,
            'presentations' => $canonicalPresentations,
        ];
        $digest = $this->digest($payload);

        return DB::transaction(function () use (
            $productId,
            $version,
            $deliveryProfile,
            $targetScope,
            $currency,
            $priceMinor,
            $availableFrom,
            $availableUntil,
            $canonicalPresentations,
            $digest,
        ): ProductCatalogVersion {
            DB::table('product_catalog_products')->insertOrIgnore([
                'product_id' => $productId,
                'created_at' => now(),
            ]);

            $product = DB::table('product_catalog_products')
                ->where('product_id', $productId)
                ->lockForUpdate()
                ->first();
            if ($product === null) {
                throw new ProductCatalogException('catalog_unavailable', 'The product catalogue is unavailable.');
            }

            $existing = DB::table('product_catalog_versions')
                ->where('product_id', $productId)
                ->where('version', $version)
                ->lockForUpdate()
                ->first();

            if ($existing !== null) {
                $storedDigest = $this->string($existing, 'payload_sha256');
                if (! hash_equals($storedDigest, $digest)) {
                    throw new ProductCatalogException(
                        'version_conflict',
                        'This product version already exists with different immutable content.',
                    );
                }

                $found = $this->find($productId, $version);
                if (! $found instanceof ProductCatalogVersion) {
                    throw new ProductCatalogException('catalog_unavailable', 'The product catalogue is unavailable.');
                }

                return $found;
            }

            DB::table('product_catalog_versions')->insert([
                'product_id' => $productId,
                'version' => $version,
                'delivery_profile' => $deliveryProfile,
                'target_scope' => $targetScope,
                'currency' => $currency,
                'price_minor' => $priceMinor,
                'available_from' => $availableFrom,
                'available_until' => $availableUntil,
                'payload_sha256' => $digest,
                'created_at' => now(),
            ]);

            $rows = [];
            foreach ($canonicalPresentations as $locale => $presentation) {
                $rows[] = [
                    'product_id' => $productId,
                    'version' => $version,
                    'locale' => $locale,
                    'name' => $presentation['name'],
                    'description' => $presentation['description'],
                    'created_at' => now(),
                ];
            }
            DB::table('product_catalog_presentations')->insert($rows);

            $found = $this->find($productId, $version);
            if (! $found instanceof ProductCatalogVersion) {
                throw new ProductCatalogException('catalog_unavailable', 'The product catalogue is unavailable.');
            }

            return $found;
        }, 3);
    }

    public function find(string $productId, int $version): ?ProductCatalogVersion
    {
        $row = DB::table('product_catalog_versions')
            ->where('product_id', $productId)
            ->where('version', $version)
            ->first();
        if ($row === null) {
            return null;
        }

        /** @var array<string, array{name:string,description:string}> $presentations */
        $presentations = [];
        foreach (DB::table('product_catalog_presentations')
            ->where('product_id', $productId)
            ->where('version', $version)
            ->orderBy('locale')
            ->get() as $presentation) {
            $presentations[$this->string($presentation, 'locale')] = [
                'name' => $this->string($presentation, 'name'),
                'description' => $this->string($presentation, 'description'),
            ];
        }
        if (array_keys($presentations) !== ['en', 'pl']) {
            throw new ProductCatalogException('catalog_integrity_failed', 'The product catalogue version is invalid.');
        }

        $storedProductId = $this->string($row, 'product_id');
        $storedVersion = $this->integer($row, 'version');
        $deliveryProfile = $this->string($row, 'delivery_profile');
        $targetScope = $this->string($row, 'target_scope');
        $currency = $this->string($row, 'currency');
        $priceMinor = $this->integer($row, 'price_minor');
        $availableFrom = $this->nullableInteger($row, 'available_from');
        $availableUntil = $this->nullableInteger($row, 'available_until');
        $storedDigest = $this->string($row, 'payload_sha256');
        $calculatedDigest = $this->digest([
            'product_id' => $storedProductId,
            'version' => $storedVersion,
            'delivery_profile' => $deliveryProfile,
            'target_scope' => $targetScope,
            'currency' => $currency,
            'price_minor' => $priceMinor,
            'available_from' => $availableFrom,
            'available_until' => $availableUntil,
            'presentations' => $presentations,
        ]);
        if (! hash_equals($storedDigest, $calculatedDigest)) {
            throw new ProductCatalogException('catalog_integrity_failed', 'The product catalogue version is invalid.');
        }

        return new ProductCatalogVersion(
            $storedProductId,
            $storedVersion,
            $deliveryProfile,
            $targetScope,
            $currency,
            $priceMinor,
            $availableFrom,
            $availableUntil,
            $storedDigest,
            $presentations,
        );
    }

    /**
     * @param  array<string, array{name:string,description:string}>  $presentations
     * @return array<string, array{name:string,description:string}>
     */
    private function validate(
        string $productId,
        int $version,
        string $deliveryProfile,
        string $targetScope,
        string $currency,
        int $priceMinor,
        array $presentations,
        ?int $availableFrom,
        ?int $availableUntil,
    ): array {
        if (preg_match('/\Aoteryn\.[a-z0-9][a-z0-9._-]{1,87}\z/D', $productId) !== 1) {
            throw new ProductCatalogException('product_id_invalid', 'The product identifier is invalid.');
        }
        if ($version < 1 || $version > 1_000_000) {
            throw new ProductCatalogException('version_invalid', 'The product version is invalid.');
        }
        if (! in_array($deliveryProfile, ProductCatalogContract::profiles(), true)) {
            throw new ProductCatalogException('delivery_profile_invalid', 'The delivery profile is invalid.');
        }
        if (! in_array($targetScope, ProductCatalogContract::targetScopes(), true)) {
            throw new ProductCatalogException('target_scope_invalid', 'The target scope is invalid.');
        }

        $allowedCurrencies = config('payments.allowed_currencies');
        if (! is_array($allowedCurrencies)
            || preg_match('/\A[A-Z]{3}\z/D', $currency) !== 1
            || ! in_array($currency, $allowedCurrencies, true)) {
            throw new ProductCatalogException('currency_unsupported', 'The product currency is not supported.');
        }
        $maximumAmount = config('payments.maximum_order_amount_minor');
        if (! is_int($maximumAmount)
            || $maximumAmount < 1
            || $priceMinor < 1
            || $priceMinor > $maximumAmount) {
            throw new ProductCatalogException('price_invalid', 'The product price is invalid.');
        }

        if (($availableFrom === null) !== ($availableUntil === null)) {
            throw new ProductCatalogException('availability_invalid', 'The product availability window is invalid.');
        }
        if ($availableFrom !== null
            && $availableUntil !== null
            && ($availableFrom < 0 || $availableUntil <= $availableFrom)) {
            throw new ProductCatalogException('availability_invalid', 'The product availability window is invalid.');
        }

        ksort($presentations);
        if (array_keys($presentations) !== ['en', 'pl']) {
            throw new ProductCatalogException('presentation_invalid', 'Exactly EN and PL product presentation is required.');
        }

        $canonical = [];
        foreach ($presentations as $locale => $presentation) {
            if (! is_array($presentation)
                || array_is_list($presentation)
                || count($presentation) !== 2
                || ! array_key_exists('name', $presentation)
                || ! array_key_exists('description', $presentation)) {
                throw new ProductCatalogException('presentation_invalid', 'The product presentation is invalid.');
            }
            $name = $this->plainText($presentation['name'], 1, 120);
            $description = $this->plainText($presentation['description'], 1, 1000);
            $canonical[$locale] = ['name' => $name, 'description' => $description];
        }

        return $canonical;
    }

    /** @param array<string, mixed> $payload */
    private function digest(array $payload): string
    {
        try {
            $encoded = json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (JsonException $exception) {
            throw new ProductCatalogException('catalog_integrity_failed', 'The product catalogue version is invalid.', $exception);
        }

        return hash('sha256', $encoded);
    }

    private function plainText(mixed $value, int $minimum, int $maximum): string
    {
        if (! is_string($value)) {
            throw new ProductCatalogException('presentation_invalid', 'The product presentation is invalid.');
        }
        $value = trim($value);
        $length = mb_strlen($value);
        if ($length < $minimum
            || $length > $maximum
            || preg_match('/[\p{Cc}\p{Cf}]/u', $value) === 1) {
            throw new ProductCatalogException('presentation_invalid', 'The product presentation is invalid.');
        }

        return $value;
    }

    private function string(object $row, string $field): string
    {
        $value = $row->{$field} ?? null;
        if (! is_string($value)) {
            throw new UnexpectedValueException("Invalid product catalogue {$field}.");
        }

        return $value;
    }

    private function integer(object $row, string $field): int
    {
        $value = $row->{$field} ?? null;
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new UnexpectedValueException("Invalid product catalogue {$field}.");
    }

    private function nullableInteger(object $row, string $field): ?int
    {
        $value = $row->{$field} ?? null;
        if ($value === null) {
            return null;
        }

        return $this->integer($row, $field);
    }
}
