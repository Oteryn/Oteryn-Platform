<?php

namespace Tests\Feature\GameAuth;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use LogicException;
use ReflectionMethod;
use Tests\TestCase;

final class NativeEvidenceMigrationRollbackTest extends TestCase
{
    use RefreshDatabase;

    public function test_activated_or_ambiguous_native_evidence_migration_refuses_destructive_rollback(): void
    {
        $identity = Identity::query()->create([
            'email' => 'native-migration-rollback@example.test',
            'password' => Hash::make('correct horse battery staple'),
        ]);
        $issuedAccountId = $identity->account_id;
        self::assertNotSame('', $issuedAccountId);

        $migration = require database_path('migrations/2026_09_09_120900_add_native_game_evidence_state.php');
        self::assertIsObject($migration);
        $down = new ReflectionMethod($migration, 'down');

        foreach ([true, 'garbage'] as $activationState) {
            config(['game-auth.native_evidence.activated' => $activationState]);

            try {
                $down->invoke($migration);
                self::fail('Native evidence identity state must not be destructively rolled back unless activation is explicitly false.');
            } catch (LogicException $exception) {
                self::assertStringContainsString('unless activation is explicitly false', $exception->getMessage());
            }

            self::assertTrue(Schema::hasColumn('identities', 'account_id'));
            self::assertTrue(Schema::hasColumn('identities', 'native_security_generation'));
            self::assertSame(
                $issuedAccountId,
                Identity::query()->findOrFail($identity->id)->account_id,
            );
        }
    }
}
