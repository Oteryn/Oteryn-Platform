<?php

namespace Tests\Feature\Accounts;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class AccountOverviewTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=account-character-projection';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    protected function setUp(): void
    {
        parent::setUp();
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::PEER],
            'game-auth.native_account_characters.source_authority' => self::SOURCE,
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => ['CN=account-character-commands'],
            'character-authority.commands.source_authority' => self::SOURCE,
            'character-authority.create.enabled' => true,
        ]);
    }

    public function test_guest_cannot_open_overview(): void
    {
        $this->get('/account')->assertRedirect('/login');
    }

    public function test_new_native_account_can_reach_creation_without_legacy_binding_or_projection(): void
    {
        $identity = $this->identity();
        $this->login($identity);

        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('id="account-characters-heading"', false)
            ->assertSee(route('account.characters.create', ['locale' => 'pl']), false)
            ->assertSee('Utwórz postać')
            ->assertSee('Lista Twoich postaci jest nieaktualna.')
            ->assertDontSee($identity->account_id)
            ->assertDontSee('Support required');
        $this->get('/account/characters/create?locale=pl')->assertOk()
            ->assertSee('name="submission"', false);
        self::assertSame(0, DB::table('identity_canary_accounts')->count());
    }

    public function test_owned_characters_come_from_native_projection_without_legacy_profile_preferences(): void
    {
        $owner = $this->identity();
        $other = $this->identity();
        $this->snapshot($owner, ['Alice Moon']);
        $this->snapshot($other, ['Private Other Hero']);
        $this->login($owner);

        $this->get('/account')->assertOk()
            ->assertSee('data-characters-state="ready"', false)
            ->assertSee('Alice Moon')
            ->assertDontSee('Private Other Hero')
            ->assertDontSee($owner->account_id)
            ->assertDontSee($other->account_id);
    }

    public function test_authoritative_empty_snapshot_shows_first_character_action(): void
    {
        $identity = $this->identity();
        $this->snapshot($identity, []);
        $this->login($identity);

        $this->get('/account')->assertOk()
            ->assertSee('No active characters are attached to this game account.')
            ->assertSee('Create your first character');
    }

    public function test_game_owned_character_limit_is_not_inferred_from_the_displayed_count(): void
    {
        $identity = $this->identity();
        $this->snapshot($identity, array_fill(0, 11, 'Native Hero'));
        $this->login($identity);

        $this->get('/account')->assertOk()
            ->assertSee(route('account.characters.create'), false)
            ->assertDontSee('active character slots')
            ->assertDontSee('has reached the limit');
    }

    #[DataProvider('nonReadyStates')]
    public function test_non_ready_projection_is_not_shown_as_empty_or_used_to_block_creation(string $state): void
    {
        $identity = $this->identity();
        $this->snapshot($identity, ['Hidden Stale Hero']);
        match ($state) {
            'missing' => DB::table('native_account_character_snapshots')->where('account_id', $identity->account_id)->delete(),
            'invalid' => DB::table('native_account_character_snapshots')->where('account_id', $identity->account_id)->update(['invalid' => true]),
            'stale' => DB::table('native_account_character_projection_state')->update(['complete_through' => now()->getTimestamp() - 300]),
            'unavailable' => config(['game-auth.native_account_characters.enabled' => false]),
            default => throw new \LogicException('Unknown test projection state.'),
        };
        $this->login($identity);

        $this->get('/account')->assertOk()
            ->assertSee('data-characters-state="'.$state.'"', false)
            ->assertSee(route('account.characters.create'), false)
            ->assertDontSee('Hidden Stale Hero')
            ->assertDontSee('No active characters are attached');
    }

    /** @return iterable<string, array{string}> */
    public static function nonReadyStates(): iterable
    {
        foreach (['missing', 'invalid', 'stale', 'unavailable'] as $state) {
            yield $state => [$state];
        }
    }

    public function test_disabled_command_surface_does_not_advertise_creation(): void
    {
        config(['character-authority.create.enabled' => false]);
        $this->login($this->identity());

        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('Tworzenie postaci jest chwilowo niedostępne.')
            ->assertDontSee(route('account.characters.create', ['locale' => 'pl']), false);
    }

    public function test_projection_database_failure_is_unavailable_and_does_not_query_legacy_data(): void
    {
        $this->login($this->identity());
        Schema::drop('native_account_character_projection_state');

        $this->get('/account')->assertOk()
            ->assertSee('data-characters-state="unavailable"', false)
            ->assertSee('Your character list is temporarily unavailable.')
            ->assertSee(route('account.characters.create'), false);
    }

    /** @param list<string> $names */
    private function snapshot(Identity $identity, array $names): void
    {
        $settings = NativeAccountCharactersSettings::current();
        self::assertNotNull($settings);
        $now = now()->getTimestamp();
        $characters = [];
        foreach ($names as $index => $name) {
            $characters[] = [
                'character_id' => sprintf('01934f10-7c04-7001-805b-%012d', $index + 1),
                'world_id' => self::WORLD,
                'name' => $name,
                'availability' => 'AVAILABLE',
            ];
        }
        app(NativeAccountCharactersIngestion::class)->snapshot($settings, self::PEER, NativeAccountCharactersSnapshot::fromWire(json_encode([
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::SOURCE,
            'account_id' => $identity->account_id,
            'projection_epoch' => '1',
            'projection_revision' => '1',
            'source_observed_at' => (string) $now,
            'characters' => $characters,
        ], JSON_THROW_ON_ERROR)), $now);
        app(NativeAccountCharactersIngestion::class)->watermark($settings, self::PEER, NativeAccountCharactersWatermark::fromWire(json_encode([
            'contract_version' => 1,
            'operation' => 'PublishProjectionWatermarkV1',
            'source_authority' => self::SOURCE,
            'projection_epoch' => '1',
            'complete_through' => (string) $now,
            'observed_at' => (string) $now,
        ], JSON_THROW_ON_ERROR)), $now);
    }

    private function identity(): Identity
    {
        return Identity::query()->create([
            'email' => uniqid('overview-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function login(Identity $identity): void
    {
        $this->post('/login', ['email' => $identity->email, 'password' => 'Correct-Horse-9!Battery'])
            ->assertRedirect(route('home'));
    }
}
