<?php

namespace Tests\Unit\ProductsEntitlements;

use App\ProductsEntitlements\Store\Delivery\StoreDeliveryRefused;
use App\ProductsEntitlements\Store\Delivery\StoreDeliveryRequest;
use App\ProductsEntitlements\Store\Delivery\StoreDeliveryResponse;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Database\QueryException;
use InvalidArgumentException;
use PHPUnit\Framework\Attributes\DataProvider;
use RuntimeException;
use Tests\TestCase;

/** Driver-independent Store delivery request parsing; runs on every test lane. */
final class StoreDeliveryRequestTest extends TestCase
{
    private const ZERO_ID = '00000000-0000-0000-0000-000000000000';

    public function test_pending_and_reconcile_pages_accept_the_first_page_and_a_cursor(): void
    {
        $requests = $this->requests();

        $first = $requests->page($this->canonical('pending-request.valid'), false);
        $next = $requests->page($this->canonical('reconcile-request.valid', [
            'after_source_revision' => '41', 'after_operation_id' => '0192b6a0-4c00-7000-8000-000000000701',
        ]), true);

        self::assertSame('0', $first['after_source_revision']);
        self::assertSame(self::ZERO_ID, $first['after_operation_id']);
        self::assertSame('41', $next['after_source_revision']);
    }

    /** @return iterable<string, array{array<string, mixed>}> */
    public static function invalidPages(): iterable
    {
        yield 'zero revision with an operation' => [['after_operation_id' => '0192b6a0-4c00-7000-8000-000000000701']];
        yield 'revision with the zero operation' => [['after_source_revision' => '7']];
        yield 'revision above uint64' => [['after_source_revision' => '18446744073709551616',
            'after_operation_id' => '0192b6a0-4c00-7000-8000-000000000701']];
    }

    /** @param array<string, mixed> $changes */
    #[DataProvider('invalidPages')]
    public function test_page_rejects_an_unpaired_or_out_of_range_cursor(array $changes): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->requests()->page($this->canonical('pending-request.valid', $changes), false);
    }

    public function test_page_rejects_non_canonical_bytes_and_reordered_members(): void
    {
        $requests = $this->requests();
        $valid = $this->canonical('pending-request.valid');

        foreach ([$valid."\n", ' '.$valid, $this->reordered('pending-request.valid')] as $raw) {
            try {
                $requests->page($raw, false);
                self::fail('A non-canonical Store page request was accepted.');
            } catch (InvalidArgumentException) {
                $this->addToAssertionCount(1);
            }
        }
    }

    public function test_page_rejects_a_schema_violation(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->requests()->page($this->canonical('pending-request.valid', ['max_entries' => 0]), false);
    }

    public function test_key_accepts_the_exact_read_request(): void
    {
        $key = $this->requests()->key($this->canonical('read-request.valid'));

        self::assertSame('OTERYN_PLATFORM_STORE_AUTHORITY', $key['issuer_authority']);
        self::assertSame('CLAIM', $key['operation_kind']);
        self::assertSame('0192b6a0-4c00-7000-8000-000000000703', $key['operation_id']);
    }

    public function test_key_rejects_reordered_members(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->requests()->key($this->reordered('read-request.valid'));
    }

    /** @return iterable<string, array{string}> */
    public static function receipts(): iterable
    {
        foreach (['receipt.deliver-item.committed', 'receipt.deliver-premium-time.committed', 'receipt.claim.committed',
            'receipt.reclaim.committed-shortfall', 'receipt.revoke-unclaimed.rejected-already-claimed',
            'receipt.flag.committed'] as $name) {
            yield $name => [$name];
        }
    }

    #[DataProvider('receipts')]
    public function test_receipt_accepts_every_contract_example(string $name): void
    {
        $raw = $this->canonical($name);

        $receipt = $this->requests()->receipt($raw);

        self::assertSame($this->fixture($name)['operation_id'], $receipt['operation_id']);
        self::assertSame($raw, StoreWire::encode($receipt, 1024));
    }

    public function test_receipt_rejects_a_committed_delivery_without_its_quantity(): void
    {
        $receipt = $this->fixture('receipt.deliver-item.committed');
        unset($receipt['delivered_quantity']);

        $this->expectException(InvalidArgumentException::class);
        $this->requests()->receipt(StoreWire::encode($receipt, 1024));
    }

    public function test_receipt_rejects_reordered_members(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->requests()->receipt($this->reordered('receipt.flag.committed'));
    }

    public function test_response_maps_success_refusal_input_and_storage_failures(): void
    {
        $response = new StoreDeliveryResponse;

        $ok = $response->handle(fn (): string => '{"contract_version":1}');
        self::assertSame(200, $ok->getStatusCode());
        self::assertSame('{"contract_version":1}', $ok->getContent());
        self::assertSame('application/json', $ok->headers->get('Content-Type'));
        self::assertSame(404, $response->handle(fn (): string => throw new StoreDeliveryRefused(404))->getStatusCode());
        self::assertSame(400, $response->handle(fn (): string => throw new InvalidArgumentException('bad'))->getStatusCode());
        $failure = new QueryException('testing', 'select 1', [], new RuntimeException('storage unavailable'));
        $unavailable = $response->handle(fn (): string => throw $failure);
        self::assertSame(503, $unavailable->getStatusCode());
        self::assertSame('', $unavailable->getContent());
    }

    private function requests(): StoreDeliveryRequest
    {
        return $this->app->make(StoreDeliveryRequest::class);
    }

    /** @param array<string, mixed> $changes */
    private function canonical(string $name, array $changes = []): string
    {
        return StoreWire::encode(array_replace($this->fixture($name), $changes), 1024);
    }

    private function reordered(string $name): string
    {
        $data = $this->fixture($name);
        $last = array_key_last($data);
        self::assertIsString($last);
        $value = $data[$last];
        unset($data[$last]);

        return StoreWire::encode([$last => $value, ...$data], 1024);
    }

    /** @return array<string, mixed> */
    private function fixture(string $name): array
    {
        $raw = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$name.'.json'));
        self::assertIsString($raw);
        /** @var array<string, mixed> $data */
        $data = json_decode($raw, true, 128, JSON_THROW_ON_ERROR);

        return $data;
    }
}
