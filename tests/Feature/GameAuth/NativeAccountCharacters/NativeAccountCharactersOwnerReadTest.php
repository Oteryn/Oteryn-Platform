<?php

namespace Tests\Feature\GameAuth\NativeAccountCharacters;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Testing\TestResponse;
use Laravel\Passport\RefreshToken;
use Laravel\Passport\Token;
use Symfony\Component\HttpFoundation\Response;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\Feature\GameAuth\OAuth\Concerns\CreatesNativeOAuthBootstrapToken;
use Tests\TestCase;

final class NativeAccountCharactersOwnerReadTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use CreatesNativeOAuthBootstrapToken;
    use RefreshDatabase;

    private const IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureEphemeralPassportKeys();
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_020));
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::IDENTITY],
            'game-auth.native_account_characters.source_authority' => self::AUTHORITY,
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
    }

    public function test_owner_read_distinguishes_missing_ready_invalid_and_stale_without_revoking_oauth(): void
    {
        $identity = $this->createOAuthIdentity();
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);
        $this->watermark();

        $missing = $this->withToken($bootstrap['access_token'])->getJson('/api/v1/game-auth/native-characters');
        $missing->assertOk()
            ->assertExactJson(['protocol_version' => 2, 'characters' => []]);
        $this->assertNoStore($missing);

        $this->snapshot($identity);
        $ready = $this->withToken($bootstrap['access_token'])->getJson('/api/v1/game-auth/native-characters');
        $ready->assertOk()
            ->assertJsonPath('protocol_version', 2)
            ->assertJsonPath('characters.0.character_id', self::CHARACTER)
            ->assertJsonPath('characters.0.world_id', self::WORLD)
            ->assertJsonPath('characters.0.name', 'Aldric')
            ->assertJsonPath('characters.0.availability', 'AVAILABLE');
        $this->assertNoStore($ready);

        $token = Token::query()->where('user_id', $identity->id)->firstOrFail();
        self::assertFalse($token->revoked);
        self::assertFalse((bool) RefreshToken::query()->where('access_token_id', $token->getKey())->value('revoked'));

        DB::table('native_account_character_snapshots')->where('account_id', $identity->account_id)->update(['invalid' => true]);
        $invalid = $this->withToken($bootstrap['access_token'])->getJson('/api/v1/game-auth/native-characters');
        $invalid->assertStatus(503)->assertContent('');
        $this->assertNoStore($invalid);

        DB::table('native_account_character_snapshots')->where('account_id', $identity->account_id)->update(['invalid' => false]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_050));
        $stale = $this->withToken($bootstrap['access_token'])->getJson('/api/v1/game-auth/native-characters');
        $stale->assertStatus(503)->assertContent('');
        $this->assertNoStore($stale);
    }

    public function test_owner_read_requires_native_client_scope_and_current_generation(): void
    {
        $identity = $this->createOAuthIdentity();
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity, []);
        $this->watermark();

        $this->withToken($bootstrap['access_token'])
            ->getJson('/api/v1/game-auth/native-characters')
            ->assertUnauthorized();

        $scoped = $this->issueNativeOAuthBootstrapToken($identity);
        DB::table('oauth_access_tokens')->where('user_id', $identity->id)->where('revoked', false)->update([
            'game_auth_generation' => $identity->game_auth_generation + 1,
        ]);

        $this->withToken($scoped['access_token'])
            ->getJson('/api/v1/game-auth/native-characters')
            ->assertUnauthorized();
    }

    public function test_owner_read_returns_only_the_callers_entries_and_refuses_an_account_below_a_raised_epoch(): void
    {
        $owner = $this->createOAuthIdentity();
        $other = Identity::query()->create([
            'email' => 'other@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $ownerToken = $this->issueNativeOAuthBootstrapToken($owner)['access_token'];
        // The bootstrap helper signs in through the browser session; start the other account's flow signed out.
        $this->post(route('identity.logout'))->assertRedirect();
        $otherToken = $this->issueNativeOAuthBootstrapToken($other)['access_token'];
        $this->watermark();
        $this->snapshot($owner);
        $this->snapshot($other, characterId: '01934f10-7c04-7001-805b-3b1122334402', name: 'Brenna');

        $this->readAs($ownerToken)
            ->assertOk()
            ->assertExactJson(['protocol_version' => 2, 'characters' => [[
                'character_id' => self::CHARACTER,
                'world_id' => self::WORLD,
                'name' => 'Aldric',
                'availability' => 'AVAILABLE',
            ]]]);
        $this->readAs($otherToken)
            ->assertOk()
            ->assertJsonCount(1, 'characters')
            ->assertJsonPath('characters.0.name', 'Brenna');

        // The owner resyncs in epoch 2 and the feed is live again; the other account is still in epoch 1.
        $this->snapshot($owner, epoch: '2');
        $this->watermark('2');
        $this->readAs($ownerToken)->assertOk()->assertJsonCount(1, 'characters');
        $below = $this->readAs($otherToken);
        $below->assertStatus(503)->assertContent('');
        $this->assertNoStore($below);
    }

    public function test_owner_read_is_unavailable_when_enabled_outside_testing_and_preproduction(): void
    {
        $identity = $this->createOAuthIdentity();
        $token = $this->issueNativeOAuthBootstrapToken($identity)['access_token'];
        $this->watermark();
        $this->snapshot($identity);

        foreach (['local', 'staging', 'production'] as $environment) {
            $this->app->detectEnvironment(fn (): string => $environment);
            $this->withToken($token)->getJson('/api/v1/game-auth/native-characters')->assertStatus(503)->assertContent('');
        }
        $this->app->detectEnvironment(fn (): string => 'testing');
    }

    private function snapshot(
        Identity $identity,
        string $epoch = '1',
        string $characterId = self::CHARACTER,
        string $name = 'Aldric',
    ): void {
        $wire = json_encode([
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::AUTHORITY,
            'account_id' => $identity->account_id,
            'projection_epoch' => $epoch,
            'projection_revision' => '1',
            'source_observed_at' => '1790000018',
            'characters' => [[
                'character_id' => $characterId,
                'world_id' => self::WORLD,
                'name' => $name,
                'availability' => 'AVAILABLE',
            ]],
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);

        $this->ingestion()->snapshot($this->settings(), self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($wire), now()->getTimestamp());
    }

    private function watermark(string $epoch = '1'): void
    {
        $wire = json_encode([
            'contract_version' => 1,
            'operation' => 'PublishProjectionWatermarkV1',
            'source_authority' => self::AUTHORITY,
            'projection_epoch' => $epoch,
            'complete_through' => '1790000015',
            'observed_at' => '1790000019',
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);

        $this->ingestion()->watermark($this->settings(), self::IDENTITY, NativeAccountCharactersWatermark::fromWire($wire), now()->getTimestamp());
    }

    private function ingestion(): NativeAccountCharactersIngestion
    {
        return app(NativeAccountCharactersIngestion::class);
    }

    private function settings(): NativeAccountCharactersSettings
    {
        return NativeAccountCharactersSettings::current()
            ?? throw new \RuntimeException('LCFA settings must be valid in the owner-read test.');
    }

    /** @return TestResponse<Response> */
    private function readAs(string $accessToken): TestResponse
    {
        // Each real request resolves its caller afresh; the test application otherwise keeps the first resolved user.
        $this->app['auth']->forgetGuards();

        return $this->withToken($accessToken)->getJson('/api/v1/game-auth/native-characters');
    }

    /** @param TestResponse<Response> $response */
    private function assertNoStore(TestResponse $response): void
    {
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
    }
}
