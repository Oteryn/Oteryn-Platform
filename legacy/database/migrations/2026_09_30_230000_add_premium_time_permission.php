<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

/**
 * Exact operator permission for oteryn.premium_time grants and revocations (contract 2.5). It is granted to
 * platform_admin only; narrower roles receive it through the role manager with audit.
 */
return new class extends Migration
{
    public function up(): void
    {
        $now = now();
        $permissionId = DB::table('admin_permissions')->insertGetId([
            'key' => 'products.premium.manage',
            'name' => 'Grant and revoke Premium time',
            'created_at' => $now,
            'updated_at' => $now,
        ]);
        $platformAdminRoleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');

        if (is_int($platformAdminRoleId) || (is_string($platformAdminRoleId) && ctype_digit($platformAdminRoleId))) {
            DB::table('admin_role_permissions')->insertOrIgnore([
                'role_id' => (int) $platformAdminRoleId,
                'permission_id' => $permissionId,
            ]);
        }
    }

    public function down(): void
    {
        $permissionId = DB::table('admin_permissions')->where('key', 'products.premium.manage')->value('id');

        if (is_int($permissionId) || (is_string($permissionId) && ctype_digit($permissionId))) {
            DB::table('admin_role_permissions')->where('permission_id', (int) $permissionId)->delete();
        }

        DB::table('admin_permissions')->where('key', 'products.premium.manage')->delete();
    }
};
