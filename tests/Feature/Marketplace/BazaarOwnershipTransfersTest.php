<?php

namespace Tests\Feature\Marketplace;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandState;
use App\CharacterAuthority\CharacterCommandWire;
use App\CharacterAuthority\CharacterTransferPurpose;
use App\CharacterAuthority\Http\CharacterCommandRoutes;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Native\BazaarOwnershipTransfers;
use App\Marketplace\Native\BazaarTransferStep;
use Closure;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Route;
use Illuminate\Testing\TestResponse;
use LogicException;
use ReflectionMethod;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class BazaarOwnershipTransfersTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=oteryn-game-character-commands';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const SELLER = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const ESCROW = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c7';

    private const BUYER = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c8';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const OTHER_CHARACTER = '01934f10-7c04-7001-805b-3b1122334402';

    private const AUCTION = 41;

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-10-11T12:00:00Z');
        Route::middleware('api')->group(fn () => CharacterCommandRoutes::register());
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_disabled_commands_fail_closed_without_recording_a_step(): void
    {
        $this->assertRefused('dependency_unavailable', fn () => $this->escrow());
        self::assertFalse(DB::table(BazaarOwnershipTransfers::TABLE)->exists());
        self::assertNull($this->transfers()->latest(self::AUCTION, CharacterTransferPurpose::ListingEscrow));
    }

    public function test_escrow_step_is_idempotent_and_bound_to_one_operation(): void
    {
        $this->enable();
        $first = $this->escrow();
        $again = $this->escrow();

        self::assertTrue($first->pending());
        self::assertSame(1, $first->attempt);
        self::assertSame($first->operationId, $again->operationId);
        self::assertNull($first->failureCode());
        self::assertSame(1, DB::table(BazaarOwnershipTransfers::TABLE)->count());
        self::assertSame(1, DB::table('character_command_intents')->count());

        $this->assertRefused('transfer_binding_conflict', fn () => $this->transfers()->advance(
            self::AUCTION, CharacterTransferPurpose::ListingEscrow, self::OTHER_CHARACTER, self::SELLER, self::ESCROW,
        ));
        $this->assertRefused('transfer_binding_conflict', fn () => $this->transfers()->advance(
            self::AUCTION, CharacterTransferPurpose::ListingEscrow, self::CHARACTER, self::SELLER, self::BUYER,
        ));
    }

    public function test_settlement_requires_committed_escrow_then_follows_game_receipts(): void
    {
        $this->enable();
        $this->assertRefused('auction_state_conflict', fn () => $this->settle());

        $escrow = $this->escrow();
        $this->assertRefused('auction_state_conflict', fn () => $this->settle());

        $this->publish($this->committedTransfer($escrow->operationId, self::SELLER, self::ESCROW))->assertOk();
        self::assertTrue($this->escrow()->committed());
        $this->assertRefused('transfer_not_retryable', fn () => $this->retryEscrow());

        $settlement = $this->settle();
        self::assertTrue($settlement->pending());
        self::assertNotSame($escrow->operationId, $settlement->operationId);

        $this->publish($this->committedTransfer($settlement->operationId, self::ESCROW, self::BUYER))->assertOk();
        $done = $this->transfers()->latest(self::AUCTION, CharacterTransferPurpose::Settlement);
        self::assertNotNull($done);
        self::assertTrue($done->committed());
        $this->assertRefused('transfer_not_retryable', fn () => $this->transfers()->retry(
            self::AUCTION, CharacterTransferPurpose::Settlement, self::CHARACTER, self::ESCROW, self::BUYER,
        ));
        $this->assertRefused('auction_state_conflict', fn () => $this->returnToSeller());
    }

    public function test_escrow_return_and_settlement_exclude_each_other(): void
    {
        $this->enable();
        $this->commitEscrow();

        $settlement = $this->settle();
        $this->assertRefused('auction_state_conflict', fn () => $this->returnToSeller());

        $this->publish($this->rejected($settlement->operationId, 'CHAR_CMD_ACCOUNT_NOT_ELIGIBLE'))->assertOk();
        $rejected = $this->settle();
        self::assertTrue($rejected->rejected());
        self::assertSame('account_not_eligible', $rejected->failureCode());
        self::assertFalse($rejected->retryable());

        $return = $this->returnToSeller();
        self::assertTrue($return->pending());
        $this->publish($this->committedTransfer($return->operationId, self::ESCROW, self::SELLER))->assertOk();
        self::assertTrue($this->returnToSeller()->committed());
    }

    public function test_retryable_rejection_opens_a_new_attempt_with_a_new_operation(): void
    {
        $this->enable();
        $first = $this->escrow();
        $this->assertRefused('transfer_not_retryable', fn () => $this->retryEscrow());

        $this->publish($this->rejected($first->operationId, 'CHAR_CMD_CHARACTER_IN_SESSION'))->assertOk();
        $rejected = $this->escrow();
        self::assertTrue($rejected->retryable());
        self::assertSame('character_online_or_session_active', $rejected->failureCode());

        $second = $this->retryEscrow();
        self::assertSame(2, $second->attempt);
        self::assertTrue($second->pending());
        self::assertNotSame($first->operationId, $second->operationId);
        self::assertSame($second->operationId, $this->escrow()->operationId);
        $this->assertRefused('transfer_not_retryable', fn () => $this->retryEscrow());
        $this->assertRefused('transfer_binding_conflict', fn () => $this->transfers()->retry(
            self::AUCTION, CharacterTransferPurpose::ListingEscrow, self::CHARACTER, self::SELLER, self::BUYER,
        ));
    }

    public function test_attempts_are_bounded(): void
    {
        $this->enable();
        $step = $this->escrow();
        for ($attempt = 1; $attempt < BazaarOwnershipTransfers::MAX_ATTEMPTS; $attempt++) {
            $this->publish($this->rejected($step->operationId, 'CHAR_CMD_INTENT_EXPIRED'))->assertOk();
            $step = $this->retryEscrow();
        }
        self::assertSame(BazaarOwnershipTransfers::MAX_ATTEMPTS, $step->attempt);

        $this->publish($this->rejected($step->operationId, 'CHAR_CMD_INTENT_EXPIRED'))->assertOk();
        $this->assertRefused('transfer_attempts_exhausted', fn () => $this->retryEscrow());
    }

    public function test_terminal_and_defect_rejections_are_never_retried(): void
    {
        $this->enable();
        $owner = $this->escrow();
        $this->publish($this->rejected($owner->operationId, 'CHAR_CMD_OWNER_MISMATCH'))->assertOk();
        $step = $this->escrow();
        self::assertSame('ownership_conflict', $step->failureCode());
        $this->assertRefused('transfer_not_retryable', fn () => $this->retryEscrow());

        $defect = $this->escrow(42);
        $this->publish($this->rejected($defect->operationId, 'CHAR_CMD_OPERATION_CONFLICT'))->assertOk();
        $step = $this->escrow(42);
        self::assertSame(BazaarTransferStep::CONTRACT_DEFECT, $step->failureCode());
        self::assertFalse($step->retryable());
    }

    public function test_expired_intent_without_receipt_stays_pending(): void
    {
        $this->enable();
        $step = $this->escrow();
        Carbon::setTestNow(Carbon::createFromTimestamp($step->expiresAtSource + 3600));

        $later = $this->escrow();
        self::assertSame(CharacterCommandState::Pending, $later->state);
        self::assertTrue($later->awaitingPastExpiry(now()->getTimestamp()));
        self::assertNull($later->failureCode());
        $this->assertRefused('transfer_not_retryable', fn () => $this->retryEscrow());
        $this->assertRefused('auction_state_conflict', fn () => $this->settle());
    }

    public function test_committed_receipt_must_match_the_recorded_binding(): void
    {
        $this->enable();
        $foreign = app(CharacterCommandClient::class)->requestTransfer(
            'foreign-1', self::CHARACTER, self::SELLER, self::BUYER, CharacterTransferPurpose::ListingEscrow,
        );
        $this->publish($this->committedTransfer($foreign->operationId, self::SELLER, self::BUYER))->assertOk();
        DB::table(BazaarOwnershipTransfers::TABLE)->insert([
            'auction_id' => self::AUCTION,
            'purpose' => CharacterTransferPurpose::ListingEscrow->value,
            'attempt' => 1,
            'operation_id' => $foreign->operationId,
            'character_id' => self::CHARACTER,
            'from_account_id' => self::SELLER,
            'to_account_id' => self::ESCROW,
            'created_at' => now(),
        ]);

        $this->assertRefused('transfer_receipt_conflict', fn () => $this->escrow());
        $this->assertRefused('transfer_receipt_conflict', fn () => $this->settle());
        self::assertSame(1, DB::table(BazaarOwnershipTransfers::TABLE)->count());
    }

    public function test_rollback_is_refused_while_transfers_exist(): void
    {
        $this->enable();
        $this->escrow();
        $migration = require database_path('migrations/2026_10_11_180000_create_character_auction_transfers.php');
        self::assertIsObject($migration);

        try {
            (new ReflectionMethod($migration, 'down'))->invoke($migration);
            self::fail('Rollback must be refused while Bazaar ownership transfers exist.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('rollback refused', $exception->getMessage());
        }
        self::assertTrue(DB::table(BazaarOwnershipTransfers::TABLE)->exists());
    }

    private function transfers(): BazaarOwnershipTransfers
    {
        return app(BazaarOwnershipTransfers::class);
    }

    private function escrow(int $auction = self::AUCTION, string $to = self::ESCROW): BazaarTransferStep
    {
        return $this->transfers()->advance($auction, CharacterTransferPurpose::ListingEscrow, self::CHARACTER, self::SELLER, $to);
    }

    private function retryEscrow(): BazaarTransferStep
    {
        return $this->transfers()->retry(self::AUCTION, CharacterTransferPurpose::ListingEscrow, self::CHARACTER, self::SELLER, self::ESCROW);
    }

    private function settle(): BazaarTransferStep
    {
        return $this->transfers()->advance(self::AUCTION, CharacterTransferPurpose::Settlement, self::CHARACTER, self::ESCROW, self::BUYER);
    }

    private function returnToSeller(): BazaarTransferStep
    {
        return $this->transfers()->advance(self::AUCTION, CharacterTransferPurpose::EscrowReturn, self::CHARACTER, self::ESCROW, self::SELLER);
    }

    private function commitEscrow(): void
    {
        $escrow = $this->escrow();
        $this->publish($this->committedTransfer($escrow->operationId, self::SELLER, self::ESCROW))->assertOk();
    }

    private function assertRefused(string $reason, Closure $step): void
    {
        try {
            $step();
            self::fail("Expected the Bazaar transfer step to be refused with {$reason}.");
        } catch (MarketplaceException $exception) {
            self::assertSame($reason, $exception->reason);
        }
    }

    private function enable(): void
    {
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode([self::PEER]),
            'character-authority.commands.source_authority' => self::SOURCE,
        ]);
    }

    /** @return TestResponse<Response> */
    private function publish(string $receipt): TestResponse
    {
        return $this->call('POST', CharacterCommandWire::RECEIPT_PATH, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ], $receipt);
    }

    private function committedTransfer(string $operationId, string $from, string $to): string
    {
        return $this->membersJson([
            ...$this->receiptHead($operationId, 'COMMITTED', 'CHAR_CMD_OK'),
            'character_id' => self::CHARACTER,
            'from_account_id' => $from,
            'to_account_id' => $to,
            'character_revision' => '9',
            'projection_epoch' => '1',
            'from_projection_revision' => '44',
            'to_projection_revision' => '12',
            'recovery_generation' => '3',
            'decided_at' => '1790000002',
        ]);
    }

    private function rejected(string $operationId, string $resultCode): string
    {
        return $this->membersJson([
            ...$this->receiptHead($operationId, 'REJECTED', $resultCode),
            'recovery_generation' => '3',
            'decided_at' => '1790000002',
        ]);
    }

    /** @return array<string, int|string> */
    private function receiptHead(string $operationId, string $outcome, string $resultCode): array
    {
        return [
            'contract_version' => 1,
            'operation' => 'PublishCharacterCommandReceiptV1',
            'source_authority' => self::SOURCE,
            'command' => CharacterCommandWire::TRANSFER,
            'operation_id' => $operationId,
            'outcome' => $outcome,
            'result_code' => $resultCode,
        ];
    }

    /** @param array<string, int|string> $members */
    private function membersJson(array $members): string
    {
        return json_encode($members, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
