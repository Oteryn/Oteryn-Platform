<?php

namespace Tests\Feature\CharacterAuthority;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterCommandWire;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/**
 * Gate U-CC9 (Game contract §4.1): the 300 s intent TTL covers 41 unseen keys, so Platform
 * defaults the unreceipted PENDING backlog to 40 and refuses any configured value above 41.
 */
final class CharacterCommandBacklogBoundTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=oteryn-game-character-commands';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    public function test_the_built_in_backlog_is_40_and_inside_the_ttl_safe_bound(): void
    {
        $this->enable();

        self::assertSame(41, CharacterCommandWire::MAX_PENDING_BACKLOG);
        self::assertSame(40, CharacterCommandSettings::current()?->maxPendingIntents);
    }

    public function test_the_ttl_safe_bound_itself_is_accepted(): void
    {
        $this->enable(41);

        self::assertSame(41, CharacterCommandSettings::current()?->maxPendingIntents);
        $this->exchange()->assertOk();
    }

    public function test_a_backlog_above_the_ttl_safe_bound_makes_the_surface_unavailable(): void
    {
        foreach ([42, '42', 256] as $value) {
            $this->enable($value);

            self::assertNull(CharacterCommandSettings::current(), 'max_pending_intents '.var_export($value, true));
            $this->exchange()->assertStatus(503);
        }

        try {
            app(CharacterCommandClient::class)->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
            self::fail('A backlog above the TTL-safe bound must not store an intent.');
        } catch (CharacterCommandUnavailable) {
            $this->assertDatabaseCount('character_command_intents', 0);
        }
    }

    private function enable(int|string|null $backlog = null): void
    {
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode([self::PEER]),
            'character-authority.commands.source_authority' => self::SOURCE,
        ]);
        if ($backlog !== null) {
            config(['character-authority.commands.max_pending_intents' => $backlog]);
        }
    }

    /** @return TestResponse<Response> */
    private function exchange(): TestResponse
    {
        return $this->call('POST', CharacterCommandWire::LIST_PATH, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ], json_encode([
            'contract_version' => 1,
            'operation' => 'ListPendingCharacterCommandsV1',
            'source_authority' => self::SOURCE,
            'after_source_revision' => '0',
            'after_operation_id' => '00000000-0000-0000-0000-000000000000',
            'max_entries' => 32,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));
    }
}
