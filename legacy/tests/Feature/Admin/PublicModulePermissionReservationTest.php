<?php

namespace Tests\Feature\Admin;

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use RuntimeException;
use Tests\TestCase;

final class PublicModulePermissionReservationTest extends TestCase
{
    use RefreshDatabase;

    /**
     * @var list<string>
     */
    private const INACTIVE_RESERVED_KEYS = [
        'portal.access',
        'portal.announcements.manage',
        'portal.settings.manage',
        'downloads.manage',
        'events.manage',
        'events.publish',
    ];

    public function test_inactive_public_module_permissions_are_registered_without_implicit_role_grants(): void
    {
        foreach (self::INACTIVE_RESERVED_KEYS as $permissionKey) {
            $this->assertDatabaseHas('admin_permissions', ['key' => $permissionKey]);
            self::assertContains($permissionKey, AdminPermission::all());
        }

        $grantedCount = DB::table('admin_role_permissions')
            ->join('admin_permissions', 'admin_permissions.id', '=', 'admin_role_permissions.permission_id')
            ->whereIn('admin_permissions.key', self::INACTIVE_RESERVED_KEYS)
            ->count();

        self::assertSame(0, $grantedCount);
    }

    public function test_support_content_permission_is_explicitly_granted_to_content_and_platform_admin_roles_only(): void
    {
        $this->assertDatabaseHas('admin_permissions', ['key' => AdminPermission::MANAGE_SUPPORT_CONTENT]);

        self::assertSame([
            AdminRoleManager::CONTENT_EDITOR,
            AdminRoleManager::PLATFORM_ADMIN,
        ], $this->roleKeysForPermission(AdminPermission::MANAGE_SUPPORT_CONTENT));
    }

    public function test_wiki_permissions_are_explicitly_granted_with_publish_restricted_to_platform_admin(): void
    {
        foreach ([
            AdminPermission::WIKI_ACCESS,
            AdminPermission::MANAGE_WIKI_ARTICLES,
            AdminPermission::MANAGE_WIKI_CATEGORIES,
        ] as $permission) {
            self::assertSame([
                AdminRoleManager::CONTENT_EDITOR,
                AdminRoleManager::PLATFORM_ADMIN,
            ], $this->roleKeysForPermission($permission));
        }

        self::assertSame([
            AdminRoleManager::PLATFORM_ADMIN,
        ], $this->roleKeysForPermission(AdminPermission::PUBLISH_WIKI));
    }

    /**
     * @return list<string>
     */
    private function roleKeysForPermission(string $permission): array
    {
        $roleKeys = DB::table('admin_role_permissions')
            ->join('admin_permissions', 'admin_permissions.id', '=', 'admin_role_permissions.permission_id')
            ->join('admin_roles', 'admin_roles.id', '=', 'admin_role_permissions.role_id')
            ->where('admin_permissions.key', $permission)
            ->orderBy('admin_roles.key')
            ->pluck('admin_roles.key')
            ->all();

        return array_values(array_map(
            static function (mixed $roleKey): string {
                if (! is_string($roleKey)) {
                    throw new RuntimeException('Expected every administrator role key to be a string.');
                }

                return $roleKey;
            },
            $roleKeys,
        ));
    }
}
