<?php

namespace Tests\Feature\Accounts;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class AccountOverviewTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        config()->set('database.connections.canary', [
            'driver' => 'sqlite',
            'database' => ':memory:',
            'prefix' => '',
            'foreign_key_constraints' => true,
        ]);

        DB::purge('canary');
        Schema::connection('canary')->create('players', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->unsignedInteger('account_id');
            $table->integer('level');
            $table->integer('vocation');
            $table->bigInteger('deletion')->default(0);
        });
    }

    protected function tearDown(): void
    {
        DB::purge('canary');

        parent::tearDown();
    }

    public function test_guest_cannot_open_overview(): void
    {
        $this->get('/account')->assertRedirect('/login');
    }

    public function test_ready_binding_is_presented_without_internal_identifiers(): void
    {
        $identity = $this->identityWithBinding(991234, IdentityCanaryAccount::STATUS_READY);
        $binding = IdentityCanaryAccount::query()->findOrFail($identity->id);
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('<h1>'.__('portal.account.heading').'</h1>', false);
        $response->assertSee('Ready');
        $response->assertSee('Your game account setup is complete and character creation is available.');
        $response->assertSee('Your characters');
        $response->assertSee('You do not have any active characters yet.');
        $response->assertDontSee((string) $binding->canary_account_id);
        $response->assertDontSee($binding->provisioning_name);
    }

    public function test_ready_account_center_lists_only_owned_active_characters_with_readable_vocations(): void
    {
        $identity = $this->identityWithBinding(991234, IdentityCanaryAccount::STATUS_READY);
        DB::connection('canary')->table('players')->insert([
            [
                'id' => 1,
                'name' => 'Active Knight',
                'account_id' => 991234,
                'level' => 120,
                'vocation' => 4,
                'deletion' => 0,
            ],
            [
                'id' => 2,
                'name' => 'Deleted Druid',
                'account_id' => 991234,
                'level' => 90,
                'vocation' => 2,
                'deletion' => 1,
            ],
            [
                'id' => 3,
                'name' => 'Other Account Hero',
                'account_id' => 112233,
                'level' => 200,
                'vocation' => 9,
                'deletion' => 0,
            ],
        ]);
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account')
            ->assertOk()
            ->assertSee('1 of 10 active character slots is in use.')
            ->assertSee('Active Knight')
            ->assertSee('Knight')
            ->assertSee('View public profile')
            ->assertDontSee('Deleted Druid')
            ->assertDontSee('Other Account Hero')
            ->assertDontSee('991234');
    }

    public function test_ready_account_center_survives_temporary_character_read_failure(): void
    {
        $identity = $this->identityWithBinding(991234, IdentityCanaryAccount::STATUS_READY);
        Schema::connection('canary')->drop('players');
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account')
            ->assertOk()
            ->assertSee('character list is temporarily unavailable')
            ->assertSee('character data cannot be read safely right now')
            ->assertDontSee('991234');
    }

    public function test_unfinished_binding_fails_closed_with_support_guidance(): void
    {
        $identity = $this->identityWithBinding(null, IdentityCanaryAccount::STATUS_PENDING);
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Support required');
        $response->assertSee('We cannot confirm a valid game account setup state.');
        $response->assertSee('Character management becomes available after the game account connection reaches the Ready state.');
        $response->assertDontSee('Setup in progress');
        $response->assertDontSee('Retry game account setup');
    }

    public function test_conflict_binding_fails_closed_with_support_guidance(): void
    {
        $identity = $this->identityWithBinding(
            null,
            IdentityCanaryAccount::STATUS_CONFLICT,
            'binding_conflict',
        );
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Support required');
        $response->assertSee('No replacement account will be created automatically.');
    }

    public function test_binding_free_identity_renders_neutral_state_without_support_message(): void
    {
        $identity = $this->identity();
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertDontSee('Support required');
        $response->assertDontSee('We cannot confirm your game account setup');
        $response->assertDontSee('Connection status');
        $response->assertDontSee('Create a character');
        $response->assertSee('Manage account security');
    }

    public function test_polish_overview_localizes_ready_empty_and_populated_states(): void
    {
        $identity = $this->identityWithBinding(991234, IdentityCanaryAccount::STATUS_READY);
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('lang="pl"', false)
            ->assertSee('Gotowe')
            ->assertSee('Konto gry jest skonfigurowane. Możesz tworzyć postacie.')
            ->assertSee('Nie masz jeszcze aktywnych postaci.')
            ->assertDontSee('Your game account setup is complete')
            ->assertDontSee('You do not have any active characters yet.');

        DB::connection('canary')->table('players')->insert([
            'id' => 1, 'name' => 'Polish Knight', 'account_id' => 991234,
            'level' => 120, 'vocation' => 4, 'deletion' => 0,
        ]);
        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('Zajęte miejsca na aktywne postacie: 1 z 10.')
            ->assertSee('Polish Knight')
            ->assertDontSee('active character slots');

        DB::connection('canary')->table('players')->insert([
            'id' => 2, 'name' => 'Polish Druid', 'account_id' => 991234,
            'level' => 90, 'vocation' => 2, 'deletion' => 0,
        ]);
        $this->get('/account?locale=pl')->assertOk()
            ->assertSee('Zajęte miejsca na aktywne postacie: 2 z 10.')
            ->assertSee('Polish Druid')
            ->assertDontSee('active character slots')
            ->assertDontSee('991234');
    }

    /** @return iterable<string, array{?string, ?string, string, string}> */
    public static function polishSetupStates(): iterable
    {
        yield 'unfinished' => [IdentityCanaryAccount::STATUS_PENDING, null, 'Wymagana pomoc techniczna', 'Nie możemy potwierdzić prawidłowego stanu konfiguracji konta gry.'];
        yield 'conflict' => [IdentityCanaryAccount::STATUS_CONFLICT, 'binding_conflict', 'Wymagana pomoc techniczna', 'Konto zastępcze nie zostanie utworzone automatycznie.'];
    }

    #[DataProvider('polishSetupStates')]
    public function test_polish_setup_messages_preserve_non_ready_state_guidance(?string $status, ?string $failureCode, string $label, string $message): void
    {
        $identity = $status === null ? $this->identity() : $this->identityWithBinding(null, $status, $failureCode);
        $this->loginAsCurrentIdentity($identity);

        $this->get('/account?locale=pl')->assertOk()
            ->assertSee($label)
            ->assertSee($message)
            ->assertSee('Twoje postacie pojawią się po zakończeniu konfiguracji konta gry.')
            ->assertDontSee('Your characters will appear')
            ->assertDontSee('>Support required<', false);
    }

    private function identityWithBinding(?int $accountId, string $status, ?string $failureCode = null): Identity
    {
        $identity = $this->identity();

        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => $accountId,
            'provisioning_name' => 'op'.substr(hash('sha256', (string) $identity->id), 0, 30),
            'canary_creation_epoch' => 1_800_000_000 + $identity->id,
            'status' => $status,
            'last_failure_code' => $failureCode,
            'ready_at' => $status === IdentityCanaryAccount::STATUS_READY ? now() : null,
        ]);

        return $identity;
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
