<?php

namespace Tests\Feature\Admin;

use App\Admin\AdminRoleManager;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use DomainException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class AdminRoleManagementTest extends TestCase
{
    use RefreshDatabase;

    public function test_first_admin_bootstrap_requires_confirmed_mfa(): void
    {
        $identity = $this->createIdentity('no-mfa@example.com', false);

        self::assertSame(1, Artisan::call('admin:bootstrap', ['email' => $identity->email]));
        self::assertSame(0, DB::table('identity_admin_roles')->count());
        self::assertSame(0, DB::table('admin_audit_events')->count());
    }

    public function test_first_admin_bootstrap_is_single_use_and_audited(): void
    {
        $first = $this->createIdentity('first-admin@example.com');
        $second = $this->createIdentity('second-admin@example.com');

        self::assertSame(0, Artisan::call('admin:bootstrap', ['email' => strtoupper($first->email)]));
        self::assertSame(1, Artisan::call('admin:bootstrap', ['email' => $second->email]));

        $platformAdminRoleId = $this->roleId(AdminRoleManager::PLATFORM_ADMIN);

        self::assertTrue(DB::table('identity_admin_roles')
            ->where('identity_id', $first->id)
            ->where('role_id', $platformAdminRoleId)
            ->exists());
        self::assertFalse(DB::table('identity_admin_roles')
            ->where('identity_id', $second->id)
            ->exists());

        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => null,
            'action' => 'admin.bootstrap_first_platform_admin',
            'target_type' => 'identity',
            'target_id' => (string) $first->id,
        ]);
    }

    public function test_role_management_route_requires_explicit_permission(): void
    {
        $actor = $this->createIdentity('content-only@example.com');
        $target = $this->createIdentity('target@example.com');
        $this->assignRoleDirectly($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.roles.store', $target), [
            'role' => AdminRoleManager::SECURITY_ADMIN,
        ])->assertForbidden();

        self::assertFalse(DB::table('identity_admin_roles')
            ->where('identity_id', $target->id)
            ->where('role_id', $this->roleId(AdminRoleManager::SECURITY_ADMIN))
            ->exists());
    }

    public function test_authorized_role_assignment_is_persisted_and_audited(): void
    {
        $actor = $this->createIdentity('security-admin@example.com');
        $target = $this->createIdentity('new-editor@example.com');
        $this->assignRoleDirectly($actor, AdminRoleManager::SECURITY_ADMIN);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.roles.store', $target), [
            'role' => AdminRoleManager::CONTENT_EDITOR,
        ])->assertRedirect(route('admin.roles.index'));

        self::assertTrue(DB::table('identity_admin_roles')
            ->where('identity_id', $target->id)
            ->where('role_id', $this->roleId(AdminRoleManager::CONTENT_EDITOR))
            ->exists());

        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'admin.role_assigned',
            'target_type' => 'identity',
            'target_id' => (string) $target->id,
        ]);
    }

    public function test_final_platform_admin_assignment_cannot_be_removed(): void
    {
        $identity = $this->createIdentity('sole-platform-admin@example.com');
        $this->assignRoleDirectly($identity, AdminRoleManager::PLATFORM_ADMIN);

        $this->expectException(DomainException::class);

        app(AdminRoleManager::class)->remove(
            $identity,
            $identity->id,
            AdminRoleManager::PLATFORM_ADMIN,
        );
    }

    public function test_platform_admin_can_be_removed_when_another_platform_admin_remains(): void
    {
        $actor = $this->createIdentity('platform-one@example.com');
        $target = $this->createIdentity('platform-two@example.com');
        $this->assignRoleDirectly($actor, AdminRoleManager::PLATFORM_ADMIN);
        $this->assignRoleDirectly($target, AdminRoleManager::PLATFORM_ADMIN);

        self::assertTrue(app(AdminRoleManager::class)->remove(
            $actor,
            $target->id,
            AdminRoleManager::PLATFORM_ADMIN,
        ));

        self::assertFalse(DB::table('identity_admin_roles')
            ->where('identity_id', $target->id)
            ->where('role_id', $this->roleId(AdminRoleManager::PLATFORM_ADMIN))
            ->exists());
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'admin.role_removed',
            'target_type' => 'identity',
            'target_id' => (string) $target->id,
        ]);
    }

    private function createIdentity(string $email, bool $confirmedMfa = true): Identity
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

    private function assignRoleDirectly(Identity $identity, string $roleKey): void
    {
        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $this->roleId($roleKey),
        ]);
    }

    private function roleId(string $roleKey): int
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if (is_int($roleId)) {
            return $roleId;
        }

        if (is_string($roleId) && ctype_digit($roleId)) {
            return (int) $roleId;
        }

        self::fail('Expected an integer-compatible administrator role id.');
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
