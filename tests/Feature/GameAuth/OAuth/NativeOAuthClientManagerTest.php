<?php

namespace Tests\Feature\GameAuth\OAuth;

use App\GameAuth\OAuth\NativeOAuthClientManager;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Testing\PendingCommand;
use Laravel\Passport\Client;
use LogicException;
use Tests\TestCase;

final class NativeOAuthClientManagerTest extends TestCase
{
    use RefreshDatabase;

    public function test_ensure_command_is_registered_and_idempotently_creates_one_public_client(): void
    {
        $this->assertArtisanSuccess($this->artisan('game-auth:oauth-client:ensure'));
        $this->assertArtisanSuccess($this->artisan('game-auth:oauth-client:ensure'));

        $client = Client::query()->sole();
        $redirectUris = $client->getAttribute('redirect_uris');

        self::assertFalse($client->confidential());
        self::assertNull($client->getAttribute('secret'));
        self::assertNull($client->getAttribute('owner_id'));
        self::assertNull($client->getAttribute('owner_type'));
        self::assertTrue($client->hasGrantType('authorization_code'));
        self::assertIsArray($redirectUris);
        self::assertSame(['http://127.0.0.1/callback'], $redirectUris);
    }

    public function test_native_redirect_configuration_fails_closed_if_it_is_not_exact_loopback_contract(): void
    {
        config(['game-auth.oauth.native_redirect_uri' => 'http://localhost/callback']);

        $this->expectException(LogicException::class);
        $this->app->make(NativeOAuthClientManager::class)->ensure();
    }

    public function test_existing_confidential_client_with_native_name_is_rejected(): void
    {
        Client::query()->create([
            'id' => 'confidential-native-client',
            'owner_id' => null,
            'owner_type' => null,
            'name' => 'Oteryn OTClient',
            'secret' => 'not-a-real-secret',
            'provider' => null,
            'redirect_uris' => ['http://127.0.0.1/callback'],
            'grant_types' => ['authorization_code'],
            'revoked' => false,
        ]);

        $this->expectException(LogicException::class);
        $this->app->make(NativeOAuthClientManager::class)->ensure();
    }

    public function test_rust_ensure_command_creates_an_idempotent_separate_public_client(): void
    {
        $this->assertArtisanSuccess($this->artisan('game-auth:oauth-client:ensure', ['--rust' => true]));
        $this->assertArtisanSuccess($this->artisan('game-auth:oauth-client:ensure', ['--rust' => true]));

        $client = Client::query()->sole();
        self::assertSame(NativeOAuthClientManager::RUST_CLIENT_NAME, $client->name);
        self::assertFalse($client->confidential());
        self::assertNull($client->getAttribute('secret'));
    }

    private function assertArtisanSuccess(PendingCommand|int $result): void
    {
        if (is_int($result)) {
            self::assertSame(0, $result);

            return;
        }

        $result->assertSuccessful();
    }
}
