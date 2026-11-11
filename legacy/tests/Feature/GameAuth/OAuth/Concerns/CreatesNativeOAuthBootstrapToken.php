<?php

namespace Tests\Feature\GameAuth\OAuth\Concerns;

use App\GameAuth\OAuth\NativeOAuthClientManager;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\Hash;
use Laravel\Passport\Client;
use Tests\TestCase;

/** @mixin TestCase */
trait CreatesNativeOAuthBootstrapToken
{
    /**
     * @param  list<string>  $scopes
     * @return array{access_token: string, refresh_token: string, client: Client}
     */
    private function issueNativeOAuthBootstrapToken(
        Identity $identity,
        array $scopes = ['game:ticket'],
        ?Client $client = null,
    ): array {
        $client ??= $this->app->make(NativeOAuthClientManager::class)->ensure();
        $verifier = rtrim(strtr(base64_encode(random_bytes(64)), '+/', '-_'), '=');
        $challenge = rtrim(strtr(base64_encode(hash('sha256', $verifier, true)), '+/', '-_'), '=');
        $redirectUri = 'http://127.0.0.1:49200/callback';
        $state = 'state-'.bin2hex(random_bytes(16));

        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));

        $authorization = $this->get('/oauth/authorize?'.http_build_query([
            'client_id' => $client->getKey(),
            'redirect_uri' => $redirectUri,
            'response_type' => 'code',
            'scope' => implode(' ', $scopes),
            'state' => $state,
            'code_challenge' => $challenge,
            'code_challenge_method' => 'S256',
        ], '', '&', PHP_QUERY_RFC3986));
        $authorization->assertOk();

        $body = $authorization->getContent();

        if (! is_string($body)) {
            self::fail('OAuth authorization response body was not a string.');
        }

        preg_match('/name="auth_token" value="([^"]+)"/', $body, $matches);
        $authToken = $matches[1] ?? null;

        if (! is_string($authToken)) {
            self::fail('OAuth authorization response did not contain auth_token.');
        }

        $approval = $this->post(route('passport.authorizations.approve'), [
            'state' => $state,
            'client_id' => $client->getKey(),
            'auth_token' => html_entity_decode($authToken, ENT_QUOTES | ENT_HTML5),
        ]);
        $location = $approval->headers->get('Location');

        if (! is_string($location)) {
            self::fail('OAuth approval did not return a redirect location.');
        }

        $queryString = parse_url($location, PHP_URL_QUERY);

        if (! is_string($queryString)) {
            self::fail('OAuth approval redirect did not contain a query string.');
        }

        parse_str($queryString, $query);
        $code = $query['code'] ?? null;

        if (! is_string($code)) {
            self::fail('OAuth approval redirect did not contain an authorization code.');
        }

        $token = $this->post('/oauth/token', [
            'grant_type' => 'authorization_code',
            'client_id' => $client->getKey(),
            'redirect_uri' => $redirectUri,
            'code' => $code,
            'code_verifier' => $verifier,
        ]);
        $token->assertOk();
        $accessToken = $token->json('access_token');
        $refreshToken = $token->json('refresh_token');

        if (! is_string($accessToken) || ! is_string($refreshToken)) {
            self::fail('OAuth token response did not contain bearer and refresh tokens.');
        }

        return [
            'access_token' => $accessToken,
            'refresh_token' => $refreshToken,
            'client' => $client,
        ];
    }

    private function createOAuthIdentity(): Identity
    {
        return Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }
}
