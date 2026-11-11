<?php

namespace Tests\Feature\GameAuth\OAuth;

use App\GameAuth\OAuth\NativeOAuthClientManager;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\TestCase;

final class NativeOAuthGrantPolicyTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use RefreshDatabase;

    private const PASSWORD = 'Correct-Horse-9!Battery';

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureEphemeralPassportKeys();
    }

    public function test_native_client_cannot_use_password_grant(): void
    {
        $identity = Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make(self::PASSWORD),
        ]);
        $client = $this->app->make(NativeOAuthClientManager::class)->ensure();

        $this->post('/oauth/token', [
            'grant_type' => 'password',
            'client_id' => $client->getKey(),
            'username' => $identity->email,
            'password' => self::PASSWORD,
            'scope' => 'game:ticket',
        ])->assertStatus(400);
    }

    public function test_unregistered_scope_is_rejected_at_authorization_boundary(): void
    {
        $identity = Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make(self::PASSWORD),
        ]);
        $client = $this->app->make(NativeOAuthClientManager::class)->ensure();
        $verifier = rtrim(strtr(base64_encode(random_bytes(64)), '+/', '-_'), '=');
        $challenge = rtrim(strtr(base64_encode(hash('sha256', $verifier, true)), '+/', '-_'), '=');
        $redirectUri = 'http://127.0.0.1:49159/callback';
        $state = 'invalid-scope-state';
        $url = '/oauth/authorize?'.http_build_query([
            'client_id' => $client->getKey(),
            'redirect_uri' => $redirectUri,
            'response_type' => 'code',
            'scope' => 'admin:all',
            'state' => $state,
            'code_challenge' => $challenge,
            'code_challenge_method' => 'S256',
        ], '', '&', PHP_QUERY_RFC3986);

        $this->post('/login', [
            'email' => $identity->email,
            'password' => self::PASSWORD,
        ])->assertRedirect(route('home'));

        $response = $this->get($url);
        $location = $response->headers->get('Location');

        if (! is_string($location)) {
            self::fail('Invalid OAuth scope did not produce an OAuth error redirect.');
        }

        self::assertStringStartsWith($redirectUri, $location);
        $queryString = parse_url($location, PHP_URL_QUERY);

        if (! is_string($queryString)) {
            self::fail('OAuth error redirect did not contain query parameters.');
        }

        parse_str($queryString, $query);
        self::assertSame('invalid_scope', $query['error'] ?? null);
        self::assertSame($state, $query['state'] ?? null);
        self::assertArrayNotHasKey('code', $query);
        self::assertSame(0, DB::table('oauth_auth_codes')->count());
    }
}
