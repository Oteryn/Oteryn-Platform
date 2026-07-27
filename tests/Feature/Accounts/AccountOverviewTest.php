<?php

namespace Tests\Feature\Accounts;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Contracts\CanaryAccountProvisioningGateway;
use App\Accounts\Exceptions\CanaryAccountProvisioningUnavailable;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

final class AccountOverviewTest extends TestCase
{
    use RefreshDatabase;

    private RecordingAccountProvisioningGateway $gateway;

    protected function setUp(): void
    {
        parent::setUp();

        $this->gateway = new RecordingAccountProvisioningGateway;
        $this->app->instance(CanaryAccountProvisioningGateway::class, $this->gateway);

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

    public function test_guest_cannot_open_overview_or_retry_provisioning(): void
    {
        $this->get('/account')->assertRedirect('/login');
        $this->post('/account/provisioning/retry')->assertRedirect('/login');
    }

    public function test_ready_binding_is_presented_without_internal_identifiers(): void
    {
        $identity = $this->identityWithBinding(991234, IdentityCanaryAccount::STATUS_READY);
        $binding = IdentityCanaryAccount::query()->findOrFail($identity->id);
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Your Oteryn account');
        $response->assertSee('Ready');
        $response->assertSee('Your game account setup is complete and character creation is available.');
        $response->assertSee('Your characters');
        $response->assertSee('You do not have any active characters yet.');
        $response->assertDontSee((string) $binding->canary_account_id);
        $response->assertDontSee($binding->provisioning_name);
        $response->assertDontSee('Retry game account setup');
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
            ->assertSee('1 of 10 active character slots are in use.')
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

    public function test_pending_binding_is_presented_as_in_progress_without_retry(): void
    {
        $identity = $this->identityWithBinding(null, IdentityCanaryAccount::STATUS_PENDING);
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Setup in progress');
        $response->assertSee('Your game account setup is still in progress.');
        $response->assertSee('Character management becomes available after the game account connection reaches the Ready state.');
        $response->assertDontSee('Retry game account setup');
    }

    public function test_dependency_failure_is_presented_as_recoverable_retry(): void
    {
        $identity = $this->identityWithBinding(
            null,
            IdentityCanaryAccount::STATUS_PENDING,
            ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE,
        );
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Setup interrupted');
        $response->assertSee('Retry game account setup');
    }

    public function test_conflict_binding_fails_closed_with_support_guidance(): void
    {
        $identity = $this->identityWithBinding(
            null,
            IdentityCanaryAccount::STATUS_CONFLICT,
            ProvisionCanaryAccount::FAILURE_BINDING_CONFLICT,
        );
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Support required');
        $response->assertSee('No replacement account will be created automatically.');
        $response->assertDontSee('Retry game account setup');
    }

    public function test_missing_binding_renders_safe_non_actionable_state(): void
    {
        $identity = $this->identity();
        $this->loginAsCurrentIdentity($identity);

        $response = $this->get('/account');

        $response->assertOk();
        $response->assertSee('Support required');
        $response->assertSee('We cannot confirm your game account setup right now.');
        $response->assertDontSee('Retry game account setup');
    }

    public function test_recoverable_retry_reuses_authoritative_persisted_intent_and_marks_binding_ready(): void
    {
        $identity = $this->identityWithBinding(
            null,
            IdentityCanaryAccount::STATUS_PENDING,
            ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE,
        );
        $binding = IdentityCanaryAccount::query()->findOrFail($identity->id);
        $this->gateway->nextAccountId = 991235;
        $this->loginAsCurrentIdentity($identity);

        $response = $this->post('/account/provisioning/retry', [
            'account_id' => 777777,
            'provisioning_name' => 'client-controlled',
        ]);

        $response->assertRedirect(route('account.overview'));
        $response->assertSessionHas('status', 'Game account setup completed.');

        $binding->refresh();
        self::assertTrue($binding->isReady());
        self::assertSame(991235, $binding->canary_account_id);
        self::assertSame([
            [
                'provisioning_name' => $binding->provisioning_name,
                'creation_epoch' => $binding->canary_creation_epoch,
            ],
        ], $this->gateway->calls);

        $overview = $this->get('/account');
        $overview->assertOk();
        $overview->assertSee('Ready');
        $overview->assertDontSee('991235');
        $overview->assertDontSee($binding->provisioning_name);
    }

    public function test_direct_retry_is_rejected_when_current_state_is_not_recoverable(): void
    {
        $identity = $this->identityWithBinding(null, IdentityCanaryAccount::STATUS_PENDING);
        $this->loginAsCurrentIdentity($identity);

        $this->post('/account/provisioning/retry')
            ->assertRedirect(route('account.overview'))
            ->assertSessionHasErrors('provisioning');

        self::assertSame([], $this->gateway->calls);
    }

    public function test_retry_dependency_failure_keeps_safe_recoverable_state(): void
    {
        $identity = $this->identityWithBinding(
            null,
            IdentityCanaryAccount::STATUS_PENDING,
            ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE,
        );
        $this->gateway->throwUnavailable = true;
        $this->loginAsCurrentIdentity($identity);

        $this->post('/account/provisioning/retry')
            ->assertRedirect(route('account.overview'))
            ->assertSessionHasErrors('provisioning');

        $binding = IdentityCanaryAccount::query()->findOrFail($identity->id);
        self::assertSame(IdentityCanaryAccount::STATUS_PENDING, $binding->status);
        self::assertSame(ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE, $binding->last_failure_code);

        $this->get('/account')
            ->assertOk()
            ->assertSee('Setup interrupted')
            ->assertSee('Retry game account setup');
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

final class RecordingAccountProvisioningGateway implements CanaryAccountProvisioningGateway
{
    /** @var list<array{provisioning_name: string, creation_epoch: int}> */
    public array $calls = [];

    public int $nextAccountId = 991235;

    public bool $throwUnavailable = false;

    public function provision(string $provisioningName, int $creationEpoch): int
    {
        $this->calls[] = [
            'provisioning_name' => $provisioningName,
            'creation_epoch' => $creationEpoch,
        ];

        if ($this->throwUnavailable) {
            throw new CanaryAccountProvisioningUnavailable('dependency unavailable');
        }

        return $this->nextAccountId;
    }
}
