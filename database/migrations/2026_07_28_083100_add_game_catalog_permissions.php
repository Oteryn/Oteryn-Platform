<?php

use App\GameCatalog\Security\GameCatalogPermission;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    public function up(): void
    {
        $roleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');
        $now = now();

        foreach (GameCatalogPermission::definitions() as $key => $name) {
            DB::table('admin_permissions')->updateOrInsert(
                ['key' => $key],
                ['name' => $name, 'updated_at' => $now, 'created_at' => $now],
            );

            $permissionId = DB::table('admin_permissions')->where('key', $key)->value('id');
            if ((is_int($roleId) || (is_string($roleId) && ctype_digit($roleId)))
                && (is_int($permissionId) || (is_string($permissionId) && ctype_digit($permissionId)))) {
                DB::table('admin_role_permissions')->insertOrIgnore([
                    'role_id' => (int) $roleId,
                    'permission_id' => (int) $permissionId,
                ]);
            }
        }
    }

    public function down(): void
    {
        $keys = array_keys(GameCatalogPermission::definitions());
        $permissionIds = DB::table('admin_permissions')->whereIn('key', $keys)->pluck('id');
        DB::table('admin_role_permissions')->whereIn('permission_id', $permissionIds)->delete();
        DB::table('admin_permissions')->whereIn('key', $keys)->delete();
    }
};
