<?php

namespace Tests\Feature\Accounts;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Route;
use Tests\TestCase;

final class AccountOverviewTest extends TestCase
{
    use RefreshDatabase;

    private const PROJECTION_IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    protected function setUp(): void
    {
        parent::setUp();

        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_020));
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::PROJECTION_IDENTITY],
            'game-auth.native_account_characters.source_authority' => self::AUTHORITY,
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
    }

    public function test_guest_cannot_open_overview(): void
    {
        $this->get('/account')->assertRedirect('/login');
    }

    public function test_provisioning_retry_route_no_longer_exists(): void
    {
        $this->post('/account/provisioning/retry')->assertNotFound();
        self::assertFalse(Route::has('account.provisioning.retry'));
    }

    public function test_overview_without_a_reported_snapshot_shows_no_characters_and_no_binding_state(): void
    {
        $identity = $this->identity();
        $this->watermark();
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('<h1>'.__('portal.account.heading').'</h1>', false);
        $response->assertSee('Ready');
        $response->assertSee('Your characters will appear after the game servers report them.');
        $response->assertDontSee('Retry game account setup');
        $response->assertDontSee('Setup in progress');
        $response->assertDontSee('Support required');
        $response->assertDontSee($identity->account_id);
    }

    public function test_overview_lists_only_the_owners_native_characters(): void
    {
        $identity = $this->identity();
        $other = $this->identity();
        $this->watermark();
        $this->snapshot($identity, [['01934f10-7c04-7001-805b-3b1122334401', 'Aldric']]);
        $this->snapshot($other, [['01934f10-7c04-7001-805b-3b1122334402', 'Brenna']]);
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account')
            ->assertOk()
            ->assertSee('1 active character.')
            ->assertSee('Aldric')
            ->assertDontSee('Brenna')
            ->assertDontSee($identity->account_id)
            ->assertDontSee(self::WORLD);
    }

    public function test_overview_lists_an_empty_snapshot_as_no_active_characters(): void
    {
        $identity = $this->identity();
        $this->watermark();
        $this->snapshot($identity, []);
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account')
            ->assertOk()
            ->assertSee('You do not have any active characters yet.');
    }

    public function test_overview_survives_a_stale_or_absent_character_feed(): void
    {
        $identity = $this->identity();
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account')
            ->assertOk()
            ->assertSee('Your character list is temporarily unavailable.')
            ->assertDontSee($identity->account_id);

        $this->watermark();
        $this->snapshot($identity, [['01934f10-7c04-7001-805b-3b1122334401', 'Aldric']]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_090));

        $this->get('/account')
            ->assertOk()
            ->assertSee('Your character list is temporarily unavailable.')
            ->assertDontSee('Aldric');
    }

    public function test_polish_overview_localizes_the_native_states(): void
    {
        $identity = $this->identity();
        $this->watermark();
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('lang="pl"', false)
            ->assertSee('Gotowe')
            ->assertSee('Konto gry jest powiązane z Twoim kontem Platformy.')
            ->assertSee('Twoje postacie pojawią się po tym, jak serwery gry je zgłoszą.')
            ->assertDontSee('Your game account is linked')
            ->assertDontSee('Your characters will appear');

        $this->snapshot($identity, [
            ['01934f10-7c04-7001-805b-3b1122334401', 'Polish Knight'],
            ['01934f10-7c04-7001-805b-3b1122334402', 'Polish Druid'],
        ]);
        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('Aktywne postacie: 2.')
            ->assertSee('Polish Knight')
            ->assertSee('Polish Druid')
            ->assertDontSee('active characters');
    }

    /** @param list<array{0: string, 1: string}> $characters */
    private function snapshot(Identity $identity, array $characters): void
    {
        $wire = json_encode([
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::AUTHORITY,
            'account_id' => $identity->account_id,
            'projection_epoch' => '1',
            'projection_revision' => '1',
            'source_observed_at' => '1790000018',
            'characters' => array_map(static fn (array $character): array => [
                'character_id' => $character[0],
                'world_id' => self::WORLD,
                'name' => $character[1],
                'availability' => 'AVAILABLE',
            ], $characters),
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);

        app(NativeAccountCharactersIngestion::class)->snapshot(
            $this->settings(),
            self::PROJECTION_IDENTITY,
            NativeAccountCharactersSnapshot::fromWire($wire),
            now()->getTimestamp(),
        );
    }

    private function watermark(): void
    {
        $wire = json_encode([
            'contract_version' => 1,
            'operation' => 'PublishProjectionWatermarkV1',
            'source_authority' => self::AUTHORITY,
            'projection_epoch' => '1',
            'complete_through' => '1790000015',
            'observed_at' => '1790000019',
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);

        app(NativeAccountCharactersIngestion::class)->watermark(
            $this->settings(),
            self::PROJECTION_IDENTITY,
            NativeAccountCharactersWatermark::fromWire($wire),
            now()->getTimestamp(),
        );
    }

    private function settings(): NativeAccountCharactersSettings
    {
        return NativeAccountCharactersSettings::current()
            ?? throw new \RuntimeException('LCFA settings must be valid in the account overview test.');
    }

    private function identity(): Identity
    {
        return Identity::query()->create([
            'email' => uniqid('account-overview-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function loginAsCurrentIdentity(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));

        $this->assertAuthenticatedAs($identity, 'web');
    }
}
