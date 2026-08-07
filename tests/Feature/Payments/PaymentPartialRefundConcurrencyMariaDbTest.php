<?php

namespace Tests\Feature\Payments;

use App\Identity\Models\Identity;
use App\Payments\Actions\CreatePaymentOrder;
use App\Payments\Actions\ProcessPaymentProviderEvent;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\Models\PaymentProviderEvent;
use App\Payments\Models\PaymentReconciliationEntry;
use Carbon\CarbonImmutable;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use PDO;
use Tests\TestCase;

final class PaymentPartialRefundConcurrencyMariaDbTest extends TestCase
{
    private const DATABASE = 'oteryn_partial_refund_concurrency_test';

    private const USER = 'oteryn_partial_refund_race';

    private const PASSWORD = 'oteryn-partial-refund-concurrency-password';

    private const SECRET = 'oteryn-partial-refund-concurrency-secret';

    private ?PDO $root = null;

    private string $rootHost = '';

    private string $rootPort = '3306';

    private string $rootPassword = '';

    protected function setUp(): void
    {
        parent::setUp();

        if (! function_exists('pcntl_fork')) {
            $this->markTestSkipped('The pcntl extension is required for partial-refund concurrency.');
        }

        $host = getenv('CANARY_PROVISIONING_INTEGRATION_HOST');
        if (! is_string($host) || $host === '') {
            $this->markTestSkipped('MariaDB integration environment is not configured.');
        }

        $port = getenv('CANARY_PROVISIONING_INTEGRATION_PORT');
        $rootPassword = getenv('CANARY_PROVISIONING_INTEGRATION_ROOT_PASSWORD');
        $this->rootHost = $host;
        $this->rootPort = is_string($port) && $port !== '' ? $port : '3306';
        $this->rootPassword = is_string($rootPassword) ? $rootPassword : '';

        $this->connectRoot();
        $this->resetDatabase();
        $this->configureConnection();

        self::assertSame(0, Artisan::call('migrate:fresh', [
            '--database' => 'mysql',
            '--force' => true,
            '--no-interaction' => true,
        ]));

        $this->root = null;

        config([
            'app.env' => 'testing',
            'payments.enabled' => true,
            'payments.provider' => DeterministicTestPaymentProvider::PROVIDER,
            'payments.allowed_currencies' => ['PLN', 'EUR'],
            'payments.maximum_order_amount_minor' => 100_000_000,
            'payments.webhook.maximum_payload_bytes' => 32_768,
            'payments.webhook.signature_tolerance_seconds' => 300,
            'payments.webhook.test_secret' => self::SECRET,
        ]);
    }

    protected function tearDown(): void
    {
        DB::purge('mysql');

        if (! $this->root instanceof PDO && $this->rootHost !== '') {
            $this->connectRoot();
        }

        if ($this->root instanceof PDO) {
            $this->root->exec('DROP DATABASE IF EXISTS `'.self::DATABASE.'`');
            $this->root->exec("DROP USER IF EXISTS '".self::USER."'@'%'");
        }

        parent::tearDown();
    }

    public function test_concurrent_distinct_partial_refunds_are_serialized_without_losing_refund_value(): void
    {
        $identity = Identity::query()->create([
            'email' => 'partial-refund-concurrency@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        $now = CarbonImmutable::parse('2026-08-07T14:30:00Z');
        $this->process(
            (string) Str::uuid(),
            'payment.succeeded',
            $order,
            1_000,
            $now,
        );

        $prefix = sys_get_temp_dir().'/oteryn-partial-refund-race-'.bin2hex(random_bytes(8));
        $goPath = $prefix.'.go';
        $pids = [];
        $resultPaths = [];

        foreach ([1 => 300, 2 => 400] as $worker => $amountMinor) {
            $readyPath = $prefix.'.'.$worker.'.ready';
            $resultPath = $prefix.'.'.$worker.'.json';
            $resultPaths[] = $resultPath;
            $eventId = (string) Str::uuid();
            $pid = pcntl_fork();

            if ($pid === -1) {
                self::fail('Unable to fork the partial-refund concurrency worker.');
            }

            if ($pid === 0) {
                DB::purge('mysql');
                touch($readyPath);
                $deadline = microtime(true) + 10;
                while (! is_file($goPath) && microtime(true) < $deadline) {
                    usleep(10_000);
                }

                $result = ['status' => 'barrier_timeout'];
                if (is_file($goPath)) {
                    try {
                        $event = $this->process(
                            $eventId,
                            'payment.partially_refunded',
                            $order,
                            $amountMinor,
                            $now,
                        );
                        $result = [
                            'status' => $event->processing_state,
                            'failure_code' => $event->failure_code,
                        ];
                    } catch (PaymentException $exception) {
                        $result = ['status' => $exception->reason];
                    }
                }

                file_put_contents($resultPath, json_encode($result, JSON_THROW_ON_ERROR));
                exit(0);
            }

            $pids[] = $pid;
        }

        $deadline = microtime(true) + 10;
        while (microtime(true) < $deadline) {
            if (is_file($prefix.'.1.ready') && is_file($prefix.'.2.ready')) {
                break;
            }
            usleep(10_000);
        }

        self::assertFileExists($prefix.'.1.ready');
        self::assertFileExists($prefix.'.2.ready');
        touch($goPath);

        foreach ($pids as $pid) {
            $waitStatus = 0;
            self::assertSame($pid, pcntl_waitpid($pid, $waitStatus));

            if (! is_int($waitStatus)) {
                self::fail('The partial-refund concurrency worker returned an invalid wait status.');
            }

            self::assertTrue(pcntl_wifexited($waitStatus));
            self::assertSame(0, pcntl_wexitstatus($waitStatus));
        }

        DB::purge('mysql');

        $statuses = [];
        foreach ($resultPaths as $resultPath) {
            self::assertFileExists($resultPath);
            $contents = file_get_contents($resultPath);
            self::assertIsString($contents);
            $decoded = json_decode($contents, true, flags: JSON_THROW_ON_ERROR);
            self::assertIsArray($decoded);
            $statuses[] = $decoded['status'] ?? null;
        }

        sort($statuses);
        self::assertSame([
            PaymentProviderEvent::STATE_PROCESSED,
            PaymentProviderEvent::STATE_PROCESSED,
        ], $statuses);

        $storedOrder = PaymentOrder::query()->findOrFail($order->id);
        self::assertSame(PaymentOrder::STATUS_PARTIALLY_REFUNDED, $storedOrder->status);
        self::assertSame(4, $storedOrder->version);
        self::assertSame(3, PaymentProviderEvent::query()->count());
        self::assertSame(4, PaymentOrderTransition::query()->count());
        self::assertSame(0, PaymentReconciliationEntry::query()->count());

        $refundTransitions = PaymentOrderTransition::query()
            ->whereNotNull('refunded_total_minor')
            ->orderBy('version')
            ->get();
        self::assertCount(2, $refundTransitions);

        $verifiedRefunded = 0;
        foreach ($refundTransitions as $transition) {
            $verifiedRefunded += (int) $transition->verified_refund_amount_minor;
        }
        self::assertSame(700, $verifiedRefunded);

        $latestRefundTransition = $refundTransitions->last();
        self::assertInstanceOf(PaymentOrderTransition::class, $latestRefundTransition);
        self::assertSame(700, $latestRefundTransition->refunded_total_minor);

        foreach (glob($prefix.'.*') ?: [] as $path) {
            @unlink($path);
        }
    }

    private function process(
        string $eventId,
        string $eventType,
        PaymentOrder $order,
        int $amountMinor,
        CarbonImmutable $now,
    ): PaymentProviderEvent {
        $payload = json_encode([
            'id' => $eventId,
            'type' => $eventType,
            'created' => $now->getTimestamp(),
            'data' => [
                'order_public_id' => $order->public_id,
                'currency' => 'PLN',
                'amount_minor' => $amountMinor,
                'provider_object_reference' => null,
            ],
        ], JSON_THROW_ON_ERROR);

        return app(ProcessPaymentProviderEvent::class)->execute(
            $payload,
            [
                DeterministicTestPaymentProvider::TIMESTAMP_HEADER => (string) $now->getTimestamp(),
                DeterministicTestPaymentProvider::SIGNATURE_HEADER => DeterministicTestPaymentProvider::signature(
                    self::SECRET,
                    $now->getTimestamp(),
                    $payload,
                ),
            ],
            $now,
        );
    }

    private function resetDatabase(): void
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $this->root->exec('DROP DATABASE IF EXISTS `'.self::DATABASE.'`');
        $this->root->exec('CREATE DATABASE `'.self::DATABASE.'` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci');
        $this->root->exec("DROP USER IF EXISTS '".self::USER."'@'%'");
        $this->root->exec(
            "CREATE USER '".self::USER."'@'%' IDENTIFIED BY '".self::PASSWORD."'",
        );
        $this->root->exec(
            'GRANT ALL PRIVILEGES ON `'.self::DATABASE."`.* TO '".self::USER."'@'%'",
        );
    }

    private function configureConnection(): void
    {
        config()->set('database.default', 'mysql');
        config()->set('database.connections.mysql', [
            'driver' => 'mysql',
            'host' => $this->rootHost,
            'port' => $this->rootPort,
            'database' => self::DATABASE,
            'username' => self::USER,
            'password' => self::PASSWORD,
            'unix_socket' => '',
            'charset' => 'utf8mb4',
            'collation' => 'utf8mb4_unicode_ci',
            'prefix' => '',
            'prefix_indexes' => true,
            'strict' => true,
            'engine' => null,
        ]);
        DB::purge('mysql');
    }

    private function connectRoot(): void
    {
        $this->root = new PDO(
            "mysql:host={$this->rootHost};port={$this->rootPort};charset=utf8mb4",
            'root',
            $this->rootPassword,
            [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION],
        );
    }
}
