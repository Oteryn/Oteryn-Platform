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
use Carbon\CarbonImmutable;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use PDO;
use Tests\TestCase;

final class PaymentEventConcurrencyMariaDbTest extends TestCase
{
    private const DATABASE = 'oteryn_payment_event_concurrency_test';

    private const USER = 'oteryn_payment_event_concurrency';

    private const PASSWORD = 'oteryn-payment-event-concurrency-password';

    private const SECRET = 'oteryn-payment-event-concurrency-secret';

    private ?PDO $root = null;

    private string $rootHost = '';

    private string $rootPort = '3306';

    private string $rootPassword = '';

    protected function setUp(): void
    {
        parent::setUp();

        if (! function_exists('pcntl_fork')) {
            $this->markTestSkipped('The pcntl extension is required for payment event concurrency.');
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

        config([
            'app.env' => 'testing',
            'payments.enabled' => false,
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

    public function test_exact_duplicate_signed_events_create_one_inbox_record_and_one_settlement_transition(): void
    {
        $identity = Identity::query()->create([
            'email' => 'payment-concurrency@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            5_000,
            (string) Str::uuid(),
        );
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');
        $eventId = (string) Str::uuid();
        $payload = json_encode([
            'id' => $eventId,
            'type' => 'payment.succeeded',
            'created' => $now->getTimestamp(),
            'data' => [
                'order_public_id' => $order->public_id,
                'provider_object_reference' => 'test_payment_concurrency',
            ],
        ], JSON_THROW_ON_ERROR);
        $headers = [
            DeterministicTestPaymentProvider::TIMESTAMP_HEADER => (string) $now->getTimestamp(),
            DeterministicTestPaymentProvider::SIGNATURE_HEADER => DeterministicTestPaymentProvider::signature(
                self::SECRET,
                $now->getTimestamp(),
                $payload,
            ),
        ];

        $prefix = sys_get_temp_dir().'/oteryn-payment-event-race-'.bin2hex(random_bytes(8));
        $goPath = $prefix.'.go';
        $pids = [];
        $resultPaths = [];

        foreach ([1, 2] as $worker) {
            $readyPath = $prefix.'.'.$worker.'.ready';
            $resultPath = $prefix.'.'.$worker.'.json';
            $resultPaths[] = $resultPath;
            $pid = pcntl_fork();

            if ($pid === -1) {
                self::fail('Unable to fork the payment event concurrency worker.');
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
                        $event = app(ProcessPaymentProviderEvent::class)->execute(
                            $payload,
                            $headers,
                            $now,
                        );
                        $result = [
                            'status' => $event->processing_state,
                            'event_id' => $event->id,
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
                self::fail('The payment event concurrency worker returned an invalid wait status.');
            }

            self::assertTrue(pcntl_wifexited($waitStatus));
            self::assertSame(0, pcntl_wexitstatus($waitStatus));
        }

        DB::purge('mysql');

        $eventIds = [];
        foreach ($resultPaths as $resultPath) {
            self::assertFileExists($resultPath);
            $contents = file_get_contents($resultPath);
            self::assertIsString($contents);
            $decoded = json_decode($contents, true, flags: JSON_THROW_ON_ERROR);
            self::assertIsArray($decoded);
            self::assertSame(PaymentProviderEvent::STATE_PROCESSED, $decoded['status'] ?? null);
            $eventIdValue = $decoded['event_id'] ?? null;
            self::assertIsInt($eventIdValue);
            $eventIds[] = $eventIdValue;
        }

        self::assertCount(1, array_unique($eventIds));
        self::assertSame(1, PaymentProviderEvent::query()->count());
        self::assertSame(2, PaymentOrderTransition::query()->count());
        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, PaymentOrder::query()->findOrFail($order->id)->status);

        foreach (glob($prefix.'.*') ?: [] as $path) {
            @unlink($path);
        }
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
