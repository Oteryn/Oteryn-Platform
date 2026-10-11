<?php

namespace Tests\Feature\Marketplace;

use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\Marketplace\Actions\CreateCharacterAuction;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Illuminate\Testing\PendingCommand;
use Tests\TestCase;

final class MarketplaceListingFreezeTest extends TestCase
    use RefreshDatabase;

    private int $auctionSequence = 0;
    use RefreshDatabase;

    public function test_frozen_listing_is_refused_without_escrow_or_database_write(): void
    {
        config()->set('marketplace.listing_frozen', true);
        $seller = $this->createIdentity('freeze-seller@example.com');
        $this->actingAsCurrent($seller);

        $this->from(route('marketplace.listing.create'))
            ->post(route('marketplace.listing.store'), [
                'player_id' => 42,
                'duration_days' => 3,
                'starting_bid' => 100,
                'request_id' => (string) Str::uuid(),
            ])
            ->assertRedirect(route('marketplace.listing.create'))
            ->assertSessionHasErrors(['marketplace']);

        self::assertSame(0, CharacterAuction::query()->count());
    }

    public function test_listing_form_redirects_to_the_account_page_while_frozen(): void
    {
        config()->set('marketplace.listing_frozen', true);
        $this->actingAsCurrent($this->createIdentity('freeze-form@example.com'));

        $this->get(route('marketplace.listing.create'))
            ->assertRedirect(route('marketplace.account'))
            ->assertSessionHasErrors(['marketplace']);
    }

    public function test_frozen_listing_action_reports_a_typed_reason(): void
    {
        config()->set('marketplace.listing_frozen', true);

        try {
            app(CreateCharacterAuction::class)->execute(
                $this->createIdentity('freeze-action@example.com'),
                42,
                3,
                100,
                null,
                (string) Str::uuid(),
            );
            self::fail('A frozen Bazaar must refuse new listings.');
        } catch (MarketplaceException $exception) {
            self::assertSame('listing_frozen', $exception->reason);
        }
    }

    public function test_drain_report_counts_non_terminal_auctions_and_exits_by_state(): void
    {
        config()->set('marketplace.listing_frozen', true);
        $seller = $this->createIdentity('drain-seller@example.com');

        $this->drain(['--report-only' => true])
            ->expectsOutputToContain('Drained: no non-terminal auctions remain.')
            ->assertExitCode(0);

        $this->createAuction($seller, CharacterAuction::STATUS_COMPLETED);
        $this->drain(['--report-only' => true])->assertExitCode(0);

        $this->createAuction($seller, CharacterAuction::STATUS_SETTLEMENT_PENDING);
        $this->createAuction($seller, CharacterAuction::STATUS_RECOVERY_REQUIRED);
        $this->drain(['--report-only' => true])
            ->expectsOutputToContain('settlement_pending: 1')
            ->expectsOutputToContain('recovery_required: 1')
            ->expectsOutputToContain('Non-terminal auctions: 2')
            ->assertExitCode(2);
    }

    public function test_drain_is_not_reported_while_listings_are_unfrozen(): void
    {
        config()->set('marketplace.listing_frozen', false);

        $this->drain(['--report-only' => true])
            ->expectsOutputToContain('New listings are not frozen; the Bazaar cannot be drained.')
            ->assertExitCode(2);
    }

    public function test_drain_rejects_an_invalid_limit(): void
    {
        $this->drain(['--limit' => '0'])->assertExitCode(1);
    }

    /**
     * @param  array<string, mixed>  $options
     */
    private function drain(array $options): PendingCommand
    {
        $command = $this->artisan('marketplace:drain', $options);
        self::assertInstanceOf(PendingCommand::class, $command);

        return $command;
    }

    private function createAuction(Identity $seller, string $status): CharacterAuction
    {
        $playerId = 5_000_000 + ++$this->auctionSequence;

        return CharacterAuction::query()->create([
            'listing_request_id' => (string) Str::uuid(),
            'seller_identity_id' => $seller->id,
            'seller_canary_account_id' => 1001,
            'escrow_canary_account_id' => 9999,
            'player_id' => $playerId,
            'active_player_id' => in_array($status, [CharacterAuction::STATUS_COMPLETED], true) ? null : $playerId,
            'player_name' => 'Drain Test '.$playerId,
            'level' => 10,
            'vocation' => 1,
            'sex' => 1,
            'character_snapshot' => ['level' => 10],
            'status' => $status,
            'saga_state' => CharacterAuction::SAGA_RECOVERY_REQUIRED,
            'duration_days' => 3,
            'starting_bid' => 100,
            'current_bid' => 0,
            'bid_count' => 0,
            'lock_version' => 1,
        ]);
    }

    private function createIdentity(string $email): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();

        return $identity;
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);
        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
