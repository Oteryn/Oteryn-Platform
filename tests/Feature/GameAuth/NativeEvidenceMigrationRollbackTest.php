<?php

namespace Tests\Feature\GameAuth;

use App\Identity\Models\Identity;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use LogicException;
use Tests\TestCase;

final class NativeEvidenceMigrationRollbackTest extends TestCase
{
    use RefreshDatabase;

    public function test_activated_native_evidence_migration_refuses_destructive_rollback(): void
    {
        config(['game-auth.native_evidence.activated' => true]);

        $identity = Identity::query()->create([
            'email' => 'native-migration-rollback@example.test',
            'password' => Hash::make('correct horse battery staple'),
        ]);
        $issuedAccountId = $identity->account_id;
        self::assertNotSame('', $issuedAccountId);

        /** @var Migration $migration */
        $migration = require database_path('migrations/2026_09_09_120900_add_native_game_evidence_state.php');

        try {
            $migration->down();
            self::fail('Activated native evidence identity state must not be destructively rolled back.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('cannot be rolled back after activation', $exception->getMessage());
        }

        self::assertTrue(Schema::hasColumn('identities', 'account_id'));
        self::assertTrue(Schema::hasColumn('identities', 'native_security_generation'));
        self::assertSame(
            $issuedAccountId,
            Identity::query()->findOrFail($identity->id)->account_id,
        );
    }
}
