<?php

namespace Tests\Feature\GameAuth\NativeLogin;

use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\Identity\Models\Identity;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use LogicException;
use ReflectionMethod;
use Tests\TestCase;

final class NativeTicketRedemptionMigrationRollbackTest extends TestCase
{
    use DatabaseMigrations;

    private const ATTEMPT = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c';

    private const WORLD = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d';

    private const CHANNEL = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e';

    protected function tearDown(): void
    {
        if ($this->app !== null && Schema::hasTable('native_admission_attempts')) {
            // Destroy only this isolated fixture before the framework's migrate-down teardown.
            DB::table('native_admission_attempts')->delete();
            DB::table('game_login_tickets')
                ->whereNull('canary_account_id')
                ->orWhereNotNull('account_id')
                ->delete();
        }
        parent::tearDown();
    }

    public function test_empty_native_additions_roll_back_and_keep_canary_tickets(): void
    {
        $identity = $this->identity();
        $canary = $this->canaryTicket($identity);
        try {
            $this->applyMigration('down');
            self::assertFalse(Schema::hasTable('native_admission_attempts'));
            self::assertFalse(Schema::hasColumn('game_login_tickets', 'account_id'));
            self::assertFalse(Schema::hasColumn('game_login_tickets', 'native_security_generation'));
            self::assertFalse(Schema::hasColumn('game_login_tickets', 'attempt_ref'));
            self::assertEquals(42, DB::table('game_login_tickets')->where('id', $canary)->value('canary_account_id'));
        } finally {
            $this->applyMigration('up');
        }
    }

    public function test_native_ticket_refuses_rollback_before_any_schema_or_row_change(): void
    {
        $identity = $this->identity();
        $this->canaryTicket($identity);
        $native = $this->nativeTicket($identity);
        $before = (array) DB::table('game_login_tickets')->where('id', $native)->first();

        $this->assertRollbackRefused();

        self::assertTrue(Schema::hasTable('native_admission_attempts'));
        self::assertTrue(Schema::hasColumn('game_login_tickets', 'account_id'));
        self::assertSame(2, DB::table('game_login_tickets')->count());
        self::assertSame($before, (array) DB::table('game_login_tickets')->where('id', $native)->first());
    }

    public function test_attempt_row_alone_refuses_rollback(): void
    {
        $identity = $this->identity();
        DB::table('native_admission_attempts')->insert([
            'attempt_ref' => self::ATTEMPT,
            'ticket_hash' => hash('sha256', 'consumed-ticket'),
            'account_id' => $identity->account_id,
            'character_id' => self::CHARACTER,
            'requested_channel_id' => null,
            'world_id' => self::WORLD,
            'channel_id' => self::CHANNEL,
            'offer_digest' => hash('sha256', 'offer'),
            'signing_input' => null,
            'key_id' => 'admission-1',
            'issued_at' => 1,
            'expires_at' => 2,
        ]);

        $this->assertRollbackRefused();

        self::assertTrue(Schema::hasTable('native_admission_attempts'));
        self::assertSame(1, DB::table('native_admission_attempts')->count());
    }

    private function assertRollbackRefused(): void
    {
        try {
            $this->applyMigration('down');
            self::fail('Native admission attempts and native tickets must survive rollback.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('before DDL', $exception->getMessage());
        }
    }

    /** @param 'up'|'down' $direction */
    private function applyMigration(string $direction): void
    {
        $migration = require database_path('migrations/2026_09_30_000100_add_native_game_login_ticket_redemption.php');
        self::assertInstanceOf(Migration::class, $migration);

        (new ReflectionMethod($migration, $direction))->invoke($migration);
    }

    private function identity(): Identity
    {
        return Identity::query()->create(['email' => 'rollback@example.test', 'password' => Hash::make('Correct-Horse-9!Battery')])->refresh();
    }

    private function canaryTicket(Identity $identity): int
    {
        return DB::table('game_login_tickets')->insertGetId([
            'ticket_hash' => hash('sha256', 'canary-ticket'),
            'identity_id' => $identity->id,
            'canary_account_id' => 42,
            'audience' => 'oteryn-game-gateway',
            'security_generation' => 1,
            'expires_at' => now()->addSeconds(60),
        ]);
    }

    private function nativeTicket(Identity $identity): int
    {
        return DB::table('game_login_tickets')->insertGetId([
            'ticket_hash' => hash('sha256', 'native-ticket'),
            'identity_id' => $identity->id,
            'canary_account_id' => null,
            'account_id' => $identity->account_id,
            'audience' => NativeGameLoginTickets::AUDIENCE,
            'security_generation' => 1,
            'native_security_generation' => 1,
            'expires_at' => now()->addSeconds(60),
        ]);
    }
}
