<?php

namespace Tests\Feature\Identity;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Contracts\CanaryAccountProvisioningGateway;
use App\Accounts\Exceptions\CanaryAccountProvisioningUnavailable;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class RegistrationTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        $this->app->instance(CanaryAccountProvisioningGateway::class, new class implements CanaryAccountProvisioningGateway
        {
            private int $nextAccountId = 1000;

            public function provision(string $provisioningName, int $creationEpoch): int
            {
                return $this->nextAccountId++;
            }
        });
    }

    public function test_registration_form_is_available_and_contains_csrf_token(): void
    {
        $response = $this->get('/register');

        $response->assertOk();
        $response->assertSee('Create an Oteryn Platform identity');
        $response->assertSee('name="_token"', false);
    }

    public function test_registration_normalizes_email_hashes_password_provisions_binding_and_records_security_events(): void
    {
        $password = 'Correct-Horse-9!Battery';

        $response = $this->post('/register', [
            'email' => '  Person@Example.COM  ',
            'password' => $password,
            'password_confirmation' => $password,
            'account_id' => 999999,
            'provisioning_name' => 'client-controlled-value',
        ]);

        $response->assertRedirect(route('identity.register.create'));

        $identity = Identity::query()->firstOrFail();
        $passwordHash = $identity->password;
        $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->firstOrFail();

        self::assertSame('person@example.com', $identity->email);
        self::assertNotSame($password, $passwordHash);
        self::assertTrue(Hash::check($password, $passwordHash));
        self::assertSame('argon2id', password_get_info($passwordHash)['algoName']);
        self::assertTrue($binding->isReady());
        self::assertSame(1000, $binding->canary_account_id);
        self::assertMatchesRegularExpression('/^op[0-9a-f]{30}$/', $binding->provisioning_name);
        self::assertNotSame('client-controlled-value', $binding->provisioning_name);
        self::assertNotSame(999999, $binding->canary_account_id);

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_REGISTERED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::CANARY_ACCOUNT_PROVISIONING_REQUESTED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::CANARY_ACCOUNT_PROVISIONING_COMPLETED,
        ]);
    }

    public function test_registration_survives_canary_dependency_failure_with_pending_retryable_intent(): void
    {
        $this->app->instance(CanaryAccountProvisioningGateway::class, new class implements CanaryAccountProvisioningGateway
        {
            public function provision(string $provisioningName, int $creationEpoch): int
            {
                throw new CanaryAccountProvisioningUnavailable('dependency unavailable');
            }
        });

        $password = 'Correct-Horse-9!Battery';

        $this->post('/register', [
            'email' => 'pending@example.com',
            'password' => $password,
            'password_confirmation' => $password,
        ])->assertRedirect(route('identity.register.create'));

        $identity = Identity::query()->where('email', 'pending@example.com')->firstOrFail();
        $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->firstOrFail();

        self::assertFalse($binding->isReady());
        self::assertSame(IdentityCanaryAccount::STATUS_PENDING, $binding->status);
        self::assertNull($binding->canary_account_id);
        self::assertSame(ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE, $binding->last_failure_code);

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::CANARY_ACCOUNT_PROVISIONING_FAILED,
        ]);
    }

    public function test_registration_rejects_case_insensitive_canonical_duplicate_email(): void
    {
        $password = 'Correct-Horse-9!Battery';

        $this->post('/register', [
            'email' => 'MixedCase@Example.com',
            'password' => $password,
            'password_confirmation' => $password,
        ])->assertRedirect();

        $this->post('/register', [
            'email' => '  mixedcase@EXAMPLE.COM ',
            'password' => 'Another-Strong-7!Password',
            'password_confirmation' => 'Another-Strong-7!Password',
        ])->assertSessionHasErrors('email');

        self::assertSame(1, Identity::query()->count());
    }

    public function test_registration_rejects_malformed_email(): void
    {
        $password = 'Correct-Horse-9!Battery';

        $this->post('/register', [
            'email' => 'not-an-email',
            'password' => $password,
            'password_confirmation' => $password,
        ])->assertSessionHasErrors('email');

        self::assertSame(0, Identity::query()->count());
    }

    public function test_registration_rejects_weak_password(): void
    {
        $this->post('/register', [
            'email' => 'person@example.com',
            'password' => 'weak',
            'password_confirmation' => 'weak',
        ])->assertSessionHasErrors('password');

        self::assertSame(0, Identity::query()->count());
    }

    public function test_registration_rejects_password_confirmation_mismatch(): void
    {
        $this->post('/register', [
            'email' => 'person@example.com',
            'password' => 'Correct-Horse-9!Battery',
            'password_confirmation' => 'Different-Horse-9!Battery',
        ])->assertSessionHasErrors('password');

        self::assertSame(0, Identity::query()->count());
    }

    public function test_registration_is_rate_limited(): void
    {
        $password = 'Correct-Horse-9!Battery';

        for ($attempt = 1; $attempt <= 5; $attempt++) {
            $this->post('/register', [
                'email' => "person{$attempt}@example.com",
                'password' => $password,
                'password_confirmation' => $password,
            ])->assertRedirect();
        }

        $this->post('/register', [
            'email' => 'person6@example.com',
            'password' => $password,
            'password_confirmation' => $password,
        ])->assertStatus(429);

        self::assertSame(5, Identity::query()->count());
    }
}
