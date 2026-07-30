<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Activation\CatalogActivationService;
use App\GameCatalog\Application\Import\CatalogImportService;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use RuntimeException;
use Tests\TestCase;

final class AdminGameCatalogTest extends TestCase
{
    use RefreshDatabase;

    public function test_admin_catalog_routes_require_confirmed_mfa_and_exact_permissions(): void
    {
        $this->get('/admin/game-catalog')->assertRedirect('/login');

        $withoutMfa = $this->createIdentity('catalog-without-mfa@example.com');
        $this->assignRole($withoutMfa, 'platform_admin');
        $this->actingAsCurrent($withoutMfa);
        $this->get('/admin/game-catalog')->assertForbidden();

        $withoutPermission = $this->createIdentity('catalog-without-permission@example.com', true);
        $this->actingAsCurrent($withoutPermission);
        $this->get('/admin/game-catalog')->assertForbidden();

        $accessOnly = $this->createIdentity('catalog-access@example.com', true);
        $this->grantPermissions($accessOnly, ['game_catalog.access']);
        $this->actingAsCurrent($accessOnly);
        $this->get('/admin/game-catalog')->assertOk();
        $this->get('/admin/game-catalog/profiles')->assertOk();
        $this->get('/admin/game-catalog/snapshots')->assertForbidden();

        $snapshotViewer = $this->createIdentity('catalog-snapshots@example.com', true);
        $this->grantPermissions($snapshotViewer, ['game_catalog.snapshots.view']);
        $this->actingAsCurrent($snapshotViewer);
        $this->get('/admin/game-catalog')->assertForbidden();
        $this->get('/admin/game-catalog/snapshots')->assertOk();
        $this->get('/admin/game-catalog/findings')->assertOk();
        $this->get('/admin/game-catalog/diff')->assertOk();
    }

    public function test_admin_catalog_inspects_snapshot_profile_findings_and_visibility_without_browser_mutation(): void
    {
        $snapshot = app(CatalogImportService::class)->import($this->fixturePath());
        $profileId = $this->createPublicProfile();
        app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'public');
        DB::table('game_catalog_validation_findings')->insert([
            'import_run_id' => null,
            'snapshot_id' => $snapshot->snapshotId,
            'severity' => 'warning',
            'code' => 'acceptance.synthetic_warning',
            'path' => '$.entities[0]',
            'message' => 'Synthetic bounded administrator inspection finding.',
            'context' => null,
            'created_at' => CarbonImmutable::now('UTC'),
        ]);

        $admin = $this->createIdentity('catalog-admin@example.com', true);
        $this->assignRole($admin, 'platform_admin');
        $this->actingAsCurrent($admin);

        $this->get('/admin/game-catalog')
            ->assertOk()
            ->assertSeeText('Game Catalog')
            ->assertSeeText('Public Game Catalog')
            ->assertDontSee('type="file"', false)
            ->assertDontSeeText('Activate snapshot');

        $this->get('/admin/game-catalog/snapshots')
            ->assertOk()
            ->assertSeeText($snapshot->contentSha256)
            ->assertSeeText('15.20');

        $this->get('/admin/game-catalog/snapshots/'.$snapshot->snapshotId)
            ->assertOk()
            ->assertSeeText('fixture-protocol')
            ->assertSeeText('item:fixture-sword')
            ->assertSeeText('outside_release')
            ->assertSeeText('partial')
            ->assertSeeText('acceptance.synthetic_warning')
            ->assertSeeText('snapshot.activate');

        $this->get('/admin/game-catalog/profiles/'.$profileId)
            ->assertOk()
            ->assertSeeText('Public Game Catalog')
            ->assertSeeText('item:fixture-sword')
            ->assertSeeText('visible')
            ->assertSeeText('outside_release')
            ->assertSeeText('snapshot.activate');

        $this->get('/admin/game-catalog/findings?severity=warning&snapshot_id='.$snapshot->snapshotId)
            ->assertOk()
            ->assertSeeText('acceptance.synthetic_warning')
            ->assertSeeText('Synthetic bounded administrator inspection finding.');

        $this->get('/admin/game-catalog/diff')
            ->assertOk()
            ->assertSeeText('Snapshot diff')
            ->assertSeeText('Compare snapshots');
    }

    public function test_admin_catalog_previews_schema_13_typed_candidate_counts_without_activation(): void
    {
        $snapshot = app(CatalogImportService::class)->import(
            base_path('tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json'),
        );
        $admin = $this->createIdentity('catalog-v13-admin@example.com', true);
        $this->assignRole($admin, 'platform_admin');
        $this->actingAsCurrent($admin);

        $this->get('/admin/game-catalog/snapshots/'.$snapshot->snapshotId)
            ->assertOk()
            ->assertSeeText('Typed candidate summary')
            ->assertSeeText('Entity npc')
            ->assertSeeText('Relation npc_buy_offer')
            ->assertSeeText('Relation npc_sell_offer')
            ->assertSeeText('Unknown or unverified entities')
            ->assertSeeText('conditional');

        self::assertSame(0, DB::table('game_catalog_profiles')->whereNotNull('active_snapshot_id')->count());
        self::assertSame(0, DB::table('game_catalog_profile_entities')->count());
        self::assertSame(0, DB::table('game_catalog_profile_relations')->count());
    }

    private function createPublicProfile(): int
    {
        $releaseId = $this->integerDatabaseValue(
            DB::table('game_catalog_releases')->where('key', '15.20')->value('id'),
            'release',
        );
        $now = CarbonImmutable::now('UTC');

        return (int) DB::table('game_catalog_profiles')->insertGetId([
            'key' => 'public',
            'name' => 'Public Game Catalog',
            'target_release_id' => $releaseId,
            'active_snapshot_id' => null,
            'protocol_profile' => 'fixture-protocol',
            'complete_only' => true,
            'completeness_policy_key' => 'complete-only',
            'availability_policy_key' => 'public-proven',
            'validation_policy_key' => 'validated-snapshot',
            'public_enabled' => true,
            'allow_backports' => false,
            'lock_version' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    private function createIdentity(string $email, bool $confirmedMfa = false): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($confirmedMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    private function assignRole(Identity $identity, string $roleKey): void
    {
        $roleId = $this->integerDatabaseValue(
            DB::table('admin_roles')->where('key', $roleKey)->value('id'),
            "role {$roleKey}",
        );

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    /** @param  list<string>  $permissions */
    private function grantPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = (int) DB::table('admin_roles')->insertGetId([
            'key' => 'catalog-role-'.$identity->id,
            'name' => 'Game Catalog test role',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($permissions as $permission) {
            $permissionId = $this->integerDatabaseValue(
                DB::table('admin_permissions')->where('key', $permission)->value('id'),
                "permission {$permission}",
            );
            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $permissionId,
            ]);
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);
        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1/minimal-snapshot.json');
    }

    private function integerDatabaseValue(mixed $value, string $description): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new RuntimeException("Expected an integer-compatible {$description} id.");
    }
}
