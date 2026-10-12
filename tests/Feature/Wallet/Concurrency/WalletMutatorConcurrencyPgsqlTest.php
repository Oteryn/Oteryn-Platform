<?php

namespace Tests\Feature\Wallet\Concurrency;

use App\Identity\Models\Identity;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use App\Wallet\WalletMutator;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\Support\Concurrency\ForkedWorkers;
use Tests\Support\Concurrency\PgsqlConcurrencyDatabase;
use Tests\TestCase;

/**
 * Closes review finding F-05 for the wallet: eight real PHP processes race WalletMutator
 * against one PostgreSQL database. Skipped unless PGSQL_INTEGRATION_HOST is configured.
 */
final class WalletMutatorConcurrencyPgsqlTest extends TestCase
{
    private const WORKERS = 8;

    private PgsqlConcurrencyDatabase $database;

    protected function setUp(): void
    {
        parent::setUp();

        $reason = PgsqlConcurrencyDatabase::unavailableReason();
        if ($reason !== null) {
            $this->markTestSkipped($reason);
        }

        $this->database = new PgsqlConcurrencyDatabase;
        $this->database->create('oteryn_wallet_concurrency_test');
    }

    protected function tearDown(): void
    {
        if (isset($this->database)) {
            $this->database->drop();
        }

        parent::tearDown();
    }

    public function test_concurrent_debits_never_overspend_or_lose_an_update(): void
    {
        $identity = $this->walletWithBalance('wallet-debit-race@example.com', 500);

        $results = ForkedWorkers::run(self::WORKERS, 'pgsql', fn (int $worker): array => $this->mutate(
            $identity->id,
            -100,
            "race:debit:{$worker}",
        ));

        $outcomes = array_count_values(array_column($results, 'outcome'));
        self::assertSame(5, $outcomes['applied'] ?? 0, json_encode($results, JSON_THROW_ON_ERROR));
        self::assertSame(3, $outcomes['insufficient_balance'] ?? 0, json_encode($results, JSON_THROW_ON_ERROR));
        self::assertSame(self::WORKERS, array_sum($outcomes));

        $wallet = WalletAccount::query()->findOrFail($identity->id);
        self::assertSame(0, $wallet->available_balance);
        self::assertSame(0, $wallet->reserved_balance);
        $this->assertLedgerMatchesBalances();
        self::assertSame(5, WalletLedgerEntry::query()->where('operation_type', 'race_mutation')->count());
    }

    public function test_concurrent_credits_and_debits_keep_ledger_equal_to_balance(): void
    {
        $identity = $this->walletWithBalance('wallet-mixed-race@example.com', 400);

        $results = ForkedWorkers::run(self::WORKERS, 'pgsql', fn (int $worker): array => $this->mutate(
            $identity->id,
            $worker % 2 === 0 ? 150 : -100,
            "race:mixed:{$worker}",
        ));

        foreach ($results as $result) {
            self::assertSame('applied', $result['outcome'], json_encode($results, JSON_THROW_ON_ERROR));
        }

        // 4 credits of 150 and 4 debits of 100 on a starting balance of 400.
        $wallet = WalletAccount::query()->findOrFail($identity->id);
        self::assertSame(400 + 4 * 150 - 4 * 100, $wallet->available_balance);
        $this->assertLedgerMatchesBalances();
        self::assertSame(1 + self::WORKERS, WalletLedgerEntry::query()->count());
    }

    public function test_concurrent_replays_of_one_idempotency_key_apply_exactly_once(): void
    {
        $identity = $this->walletWithBalance('wallet-replay-race@example.com', 1_000);

        $results = ForkedWorkers::run(self::WORKERS, 'pgsql', fn (int $worker): array => $this->mutate(
            $identity->id,
            -100,
            'race:replay:single-key',
        ));

        $outcomes = array_count_values(array_column($results, 'outcome'));
        self::assertSame(1, $outcomes['applied'] ?? 0, json_encode($results, JSON_THROW_ON_ERROR));
        self::assertSame(self::WORKERS - 1, $outcomes['replayed'] ?? 0, json_encode($results, JSON_THROW_ON_ERROR));

        self::assertSame(900, WalletAccount::query()->findOrFail($identity->id)->available_balance);
        $this->assertLedgerMatchesBalances();
        self::assertSame(1, WalletLedgerEntry::query()->where('idempotency_key', 'race:replay:single-key')->count());
    }

    /** @return array<string, string> */
    private function mutate(int $identityId, int $delta, string $key): array
    {
        try {
            $applied = DB::transaction(function () use ($identityId, $delta, $key): bool {
                $mutator = new WalletMutator;
                $wallet = $mutator->lock($identityId);

                return $mutator->applyLocked($wallet, 'race_mutation', $delta, 0, $key);
            });

            return ['outcome' => $applied ? 'applied' : 'replayed'];
        } catch (MarketplaceException $exception) {
            return ['outcome' => $exception->reason];
        }
    }

    private function walletWithBalance(string $email, int $balance): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        DB::transaction(function () use ($identity, $balance): void {
            $mutator = new WalletMutator;
            $mutator->applyLocked($mutator->lock($identity->id), 'seed_credit', $balance, 0, "seed:{$identity->id}");
        });

        return $identity;
    }

    private function assertLedgerMatchesBalances(): void
    {
        foreach (WalletAccount::query()->get() as $wallet) {
            self::assertGreaterThanOrEqual(0, $wallet->available_balance);
            self::assertGreaterThanOrEqual(0, $wallet->reserved_balance);
            self::assertSame(
                (int) WalletLedgerEntry::query()->where('identity_id', $wallet->identity_id)->sum('available_delta'),
                $wallet->available_balance,
            );
            self::assertSame(
                (int) WalletLedgerEntry::query()->where('identity_id', $wallet->identity_id)->sum('reserved_delta'),
                $wallet->reserved_balance,
            );
        }
    }
}
