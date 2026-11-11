<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    public function up(): void
    {
        $now = now();
        $permissionId = DB::table('admin_permissions')->insertGetId([
            'key' => 'marketplace.manage',
            'name' => 'Manage Character Bazaar wallets and recovery',
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
        $permissionId = DB::table('admin_permissions')->where('key', 'marketplace.manage')->value('id');

        if (is_int($permissionId) || (is_string($permissionId) && ctype_digit($permissionId))) {
            DB::table('admin_role_permissions')->where('permission_id', (int) $permissionId)->delete();
        }

        DB::table('admin_permissions')->where('key', 'marketplace.manage')->delete();
    }
};
