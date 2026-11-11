<?php

namespace Tests\Feature\Identity\Recovery;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class PasswordChangeTest extends TestCase
{
    use RefreshDatabase;

    public function test_password_change_form_requires_authentication(): void
    {
        $this->get('/password/change')
            ->assertRedirect(route('identity.login.create'));
    }

    public function test_wrong_current_password_is_rejected_without_revoking_session(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $this->put('/password/change', [
            'current_password' => 'Wrong-Horse-9!Battery',
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'New-Correct-8!Password',
        ])->assertSessionHasErrors('current_password');

        $fresh = Identity::query()->findOrFail($identity->id);
        self::assertTrue(Hash::check('Correct-Horse-9!Battery', $fresh->password));
        self::assertSame(0, $fresh->web_session_generation);
        self::assertSame(0, $fresh->game_auth_generation);
        $this->assertAuthenticatedAs($identity, 'web');
    }

    public function test_successful_password_change_revokes_sessions_logs_out_current_browser_and_audits(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $this->put('/password/change', [
            'current_password' => 'Correct-Horse-9!Battery',
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'New-Correct-8!Password',
        ])->assertRedirect(route('identity.login.create'));

        $this->assertGuest('web');
        $fresh = Identity::query()->findOrFail($identity->id);
        self::assertTrue(Hash::check('New-Correct-8!Password', $fresh->password));
        self::assertFalse(Hash::check('Correct-Horse-9!Battery', $fresh->password));
        self::assertSame(1, $fresh->web_session_generation);
        self::assertSame(1, $fresh->game_auth_generation);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_PASSWORD_CHANGED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSIONS_REVOKED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_GAME_AUTHORIZATIONS_REVOKED,
        ]);

        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertSessionHasErrors('email');

        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'New-Correct-8!Password',
        ])->assertRedirect(route('home'));
        $this->assertAuthenticated('web');
    }

    public function test_password_change_rejects_weak_mismatched_and_reused_current_password(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $this->put('/password/change', [
            'current_password' => 'Correct-Horse-9!Battery',
            'password' => 'weak',
            'password_confirmation' => 'weak',
        ])->assertSessionHasErrors('password');

        $this->put('/password/change', [
            'current_password' => 'Correct-Horse-9!Battery',
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'Different-Correct-8!Password',
        ])->assertSessionHasErrors('password');

        $this->put('/password/change', [
            'current_password' => 'Correct-Horse-9!Battery',
            'password' => 'Correct-Horse-9!Battery',
            'password_confirmation' => 'Correct-Horse-9!Battery',
        ])->assertSessionHasErrors('password');

        $fresh = Identity::query()->findOrFail($identity->id);
        self::assertTrue(Hash::check('Correct-Horse-9!Battery', $fresh->password));
        self::assertSame(0, $fresh->web_session_generation);
        self::assertSame(0, $fresh->game_auth_generation);
    }

    public function test_password_change_is_rate_limited(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        for ($attempt = 1; $attempt <= 5; $attempt++) {
            $this->put('/password/change', [
                'current_password' => "Wrong-Horse-{$attempt}!Battery",
                'password' => 'New-Correct-8!Password',
                'password_confirmation' => 'New-Correct-8!Password',
            ])->assertSessionHasErrors('current_password');
        }

        $this->put('/password/change', [
            'current_password' => 'Wrong-Horse-6!Battery',
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'New-Correct-8!Password',
        ])->assertStatus(429);
    }

    private function createIdentity(): Identity
    {
        return Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function login(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));

        $this->assertAuthenticatedAs($identity, 'web');
    }
}
