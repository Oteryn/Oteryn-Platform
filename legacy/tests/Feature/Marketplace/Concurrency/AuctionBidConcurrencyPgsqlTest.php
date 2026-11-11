<?php

namespace Tests\Feature\Marketplace\Concurrency;

use App\Marketplace\Actions\PlaceAuctionBid;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Illuminate\Support\Str;
use Tests\Support\Concurrency\BazaarBidFixtures;
use Tests\Support\Concurrency\ForkedWorkers;
use Tests\Support\Concurrency\PgsqlConcurrencyDatabase;
use Tests\TestCase;

/**
 * Closes review finding F-05 for Bazaar bids: eight real PHP processes race PlaceAuctionBid
 * against one PostgreSQL database. Skipped unless PGSQL_INTEGRATION_HOST is configured.
 */
final class AuctionBidConcurrencyPgsqlTest extends TestCase
{
    private const WORKERS = 8;

    private const FUNDS = 10_000;

    private PgsqlConcurrencyDatabase $database;

    protected function setUp(): void
    {
        parent::setUp();

        $reason = PgsqlConcurrencyDatabase::unavailableReason();
        if ($reason !== null) {
            $this->markTestSkipped($reason);
        }

        $this->database = new PgsqlConcurrencyDatabase;
        $this->database->create('oteryn_bid_concurrency_test');
    }

    protected function tearDown(): void
    {
        if (isset($this->database)) {
            $this->database->drop();
        }

        parent::tearDown();
    }

    public function test_concurrent_bids_leave_one_leading_bid_and_release_each_outbid_reservation_once(): void
    {
        $auction = BazaarBidFixtures::auction(100);
        $bidders = BazaarBidFixtures::bidders(self::WORKERS, self::FUNDS);
        $requestIds = array_map(static fn (): string => (string) Str::uuid(), $bidders);

        $results = ForkedWorkers::run(self::WORKERS, 'pgsql', function (int $worker) use ($auction, $bidders, $requestIds): array {
            try {
                // Worker n bids 200 + n * 100, so the highest bid (900) must always win.
                app(PlaceAuctionBid::class)->execute($bidders[$worker], $auction, 200 + $worker * 100, $requestIds[$worker]);

                return ['outcome' => 'accepted'];
            } catch (MarketplaceException $exception) {
                return ['outcome' => $exception->reason];
            }
        });

        $dump = json_encode($results, JSON_THROW_ON_ERROR);
        $accepted = count(array_filter($results, static fn (array $r): bool => $r['outcome'] === 'accepted'));
        foreach ($results as $result) {
            self::assertContains($result['outcome'], ['accepted', 'bid_too_low'], $dump);
        }
        self::assertGreaterThanOrEqual(1, $accepted, $dump);

        $leading = AuctionBid::query()->where('auction_id', $auction->id)->where('status', AuctionBid::STATUS_LEADING)->get();
        self::assertCount(1, $leading, 'Exactly one bid may lead the auction.');
        $leadingBid = $leading->firstOrFail();
        self::assertSame(900, $leadingBid->amount);
        self::assertSame($bidders[self::WORKERS - 1]->id, $leadingBid->bidder_identity_id);

        $fresh = CharacterAuction::query()->findOrFail($auction->id);
        self::assertSame(900, $fresh->current_bid);
        self::assertSame($bidders[self::WORKERS - 1]->id, $fresh->highest_bidder_identity_id);
        self::assertSame($accepted, $fresh->bid_count);
        self::assertSame($accepted, AuctionBid::query()->where('auction_id', $auction->id)->count());
        self::assertSame($accepted - 1, AuctionBid::query()->where('status', AuctionBid::STATUS_OUTBID)->count());

        // Every accepted bid reserved once; every outbid reservation was released once.
        self::assertSame($accepted, WalletLedgerEntry::query()->where('operation_type', 'auction_bid_reserved')->count());
        self::assertSame($accepted - 1, WalletLedgerEntry::query()->where('operation_type', 'auction_bid_released')->count());
        self::assertSame(
            $accepted - 1,
            WalletLedgerEntry::query()->where('operation_type', 'auction_bid_released')->distinct()->count('idempotency_key'),
        );

        $reservedTotal = 0;
        foreach ($bidders as $bidder) {
            $wallet = WalletAccount::query()->findOrFail($bidder->id);
            $reservedTotal += $wallet->reserved_balance;
            self::assertSame(self::FUNDS, $wallet->available_balance + $wallet->reserved_balance);
            self::assertSame(
                $wallet->available_balance,
                self::FUNDS + (int) WalletLedgerEntry::query()->where('identity_id', $bidder->id)->sum('available_delta'),
            );
            self::assertSame(
                $wallet->reserved_balance,
                (int) WalletLedgerEntry::query()->where('identity_id', $bidder->id)->sum('reserved_delta'),
            );
        }
        self::assertSame(900, $reservedTotal, 'Only the leading bid may hold a reservation.');
    }

    public function test_concurrent_replays_of_one_bid_request_create_one_bid_and_one_reservation(): void
    {
        $auction = BazaarBidFixtures::auction(100);
        $bidder = BazaarBidFixtures::bidders(1, self::FUNDS)[0];
        $requestId = (string) Str::uuid();

        $results = ForkedWorkers::run(self::WORKERS, 'pgsql', function () use ($auction, $bidder, $requestId): array {
            try {
                app(PlaceAuctionBid::class)->execute($bidder, $auction, 300, $requestId);

                return ['outcome' => 'accepted'];
            } catch (MarketplaceException $exception) {
                return ['outcome' => $exception->reason];
            }
        });

        foreach ($results as $result) {
            self::assertSame('accepted', $result['outcome'], json_encode($results, JSON_THROW_ON_ERROR));
        }

        self::assertSame(1, AuctionBid::query()->where('request_id', $requestId)->count());
        self::assertSame(1, CharacterAuction::query()->findOrFail($auction->id)->bid_count);
        self::assertSame(1, WalletLedgerEntry::query()->where('operation_type', 'auction_bid_reserved')->count());
        $wallet = WalletAccount::query()->findOrFail($bidder->id);
        self::assertSame(300, $wallet->reserved_balance);
        self::assertSame(self::FUNDS - 300, $wallet->available_balance);
    }
}
