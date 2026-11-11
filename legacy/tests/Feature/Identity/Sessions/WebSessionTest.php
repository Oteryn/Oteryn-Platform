<?php

namespace Tests\Feature\Identity\Sessions;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use App\Identity\Models\IdentityWebSession;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Auth;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class WebSessionTest extends TestCase
{
    use RefreshDatabase;

    public function test_login_form_is_available_and_contains_csrf_token(): void
    {
        $response = $this->get('/login');

        $response->assertOk();
        $response->assertSee('Sign in to Oteryn Platform');
        $response->assertSee('name="_token"', false);
    }

    public function test_login_normalizes_email_and_records_session_state_and_audit_event(): void
    {
        $identity = $this->createIdentity();

        $response = $this->post('/login', [
            'email' => '  PERSON@EXAMPLE.COM  ',
            'password' => 'Correct-Horse-9!Battery',
        ]);

        $response->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');
        $response->assertSessionHas(WebSessionState::GENERATION_KEY, 0);

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_LOGIN_SUCCEEDED,
        ]);
    }

    public function test_invalid_credentials_use_same_public_error_for_known_and_unknown_identity(): void
    {
        $this->createIdentity();

        $knownIdentityResponse = $this->post('/login', [
            'email' => 'person@example.com',
            'password' => 'Wrong-Horse-9!Battery',
        ]);

        $knownIdentityResponse->assertSessionHasErrors([
            'email' => 'The provided credentials are invalid.',
        ]);
        $this->assertGuest('web');

        $unknownIdentityResponse = $this->post('/login', [
            'email' => 'unknown@example.com',
            'password' => 'Wrong-Horse-9!Battery',
        ]);

        $unknownIdentityResponse->assertSessionHasErrors([
            'email' => 'The provided credentials are invalid.',
        ]);
        $this->assertGuest('web');
    }

    public function test_disabled_identity_cannot_log_in(): void
    {
        $identity = $this->createIdentity();
        $identity->forceFill(['disabled_at' => now()])->save();

        $response = $this->post('/login', [
            'email' => 'person@example.com',
            'password' => 'Correct-Horse-9!Battery',
        ]);

        $response->assertSessionHasErrors([
            'email' => 'The provided credentials are invalid.',
        ]);
        $this->assertGuest('web');
    }

    public function test_login_is_rate_limited_per_canonical_identity_and_source_ip(): void
    {
        $this->createIdentity();

        for ($attempt = 1; $attempt <= 5; $attempt++) {
            $this->post('/login', [
                'email' => ' PERSON@example.com ',
                'password' => "Wrong-Horse-{$attempt}!Battery",
            ])->assertSessionHasErrors('email');
        }

        $this->post('/login', [
            'email' => 'person@EXAMPLE.com',
            'password' => 'Wrong-Horse-6!Battery',
        ])->assertStatus(429);
    }

    public function test_login_source_is_rate_limited_across_distinct_identity_keys(): void
    {
        for ($attempt = 1; $attempt <= 20; $attempt++) {
            $this->post('/login', [
                'email' => "unknown{$attempt}@example.com",
                'password' => 'Wrong-Horse-9!Battery',
            ])->assertSessionHasErrors('email');
        }

        $this->post('/login', [
            'email' => 'unknown21@example.com',
            'password' => 'Wrong-Horse-9!Battery',
        ])->assertStatus(429);
    }

    public function test_logout_invalidates_current_session_and_records_audit_event(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $response = $this->post('/logout');

        $response->assertRedirect(route('home'));
        $this->assertGuest('web');
        $response->assertSessionMissing(WebSessionState::GENERATION_KEY);

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_LOGGED_OUT,
        ]);
    }

    public function test_revoking_web_sessions_rejects_an_existing_session_on_next_request(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $generation = (new RevokeIdentityWebSessions(new SecurityEventRecorder))->execute($identity);

        self::assertSame(1, $generation);

        $this->get('/')->assertOk();
        $this->assertGuest('web');

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSIONS_REVOKED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSION_REJECTED,
        ]);
    }

    public function test_revoked_registered_session_is_rejected_before_a_protected_route_runs(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $registeredSessionId = session()->get(WebSessionState::REGISTRY_ID_KEY);
        self::assertIsString($registeredSessionId);
        IdentityWebSession::query()
            ->whereKey($registeredSessionId)
            ->update(['revoked_at' => now(), 'updated_at' => now()]);

        $this->get(route('account.overview'))
            ->assertRedirect(route('identity.login.create'));
        $this->assertGuest('web');

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSION_REJECTED,
        ]);
    }

    public function test_authenticated_session_without_generation_marker_fails_closed(): void
    {
        $identity = $this->createIdentity();

        Auth::guard('web')->setUser($identity);
        Auth::shouldUse('web');

        $this->get('/')->assertOk();
        $this->assertGuest('web');
    }

    public function test_disabling_identity_rejects_an_existing_web_session(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);

        $identity->forceFill(['disabled_at' => now()])->save();

        $this->get('/')->assertOk();
        $this->assertGuest('web');

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSION_REJECTED,
        ]);
    }

    public function test_session_cookie_security_defaults_are_preserved(): void
    {
        self::assertTrue((bool) config('session.http_only'));
        self::assertSame('lax', config('session.same_site'));
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
