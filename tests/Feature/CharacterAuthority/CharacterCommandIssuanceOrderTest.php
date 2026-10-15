<?php

namespace Tests\Feature\CharacterAuthority;

use App\CharacterAuthority\CharacterCommandClient;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

/**
 * Game U-CC9: issued source_revisions rise strictly on every engine. The refusal of an intent
 * drawn below a stored key, and the race against a forward-only consumer, are exercised on
 * PostgreSQL in Concurrency\CharacterCommandIssuanceOrderPgsqlTest (SQLite and MariaDB never
 * draw a row id below the table's maximum).
 */
final class CharacterCommandIssuanceOrderTest extends TestCase
{
    use RefreshDatabase;

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-10-15T12:00:00Z');
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode(['CN=oteryn-game-character-commands']),
            'character-authority.commands.source_authority' => 'oteryn:character-authority:primary',
        ]);
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_issued_source_revisions_rise_strictly(): void
    {
        $ids = [];
        foreach (['a', 'b', 'c', 'd'] as $key) {
            $operationId = $this->client()->requestCreate('submit-'.$key, self::ACCOUNT, self::WORLD, 'Aldric')->operationId;
            $id = DB::table('character_command_intents')->where('operation_id', $operationId)->value('id');
            self::assertIsInt($id);
            $ids[] = $id;
        }

        $sorted = $ids;
        sort($sorted);
        self::assertSame($sorted, $ids);
        self::assertSame(count($ids), count(array_unique($ids)));
    }

    private function client(): CharacterCommandClient
    {
        return app(CharacterCommandClient::class);
    }
}
