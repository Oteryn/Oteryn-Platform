<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    /** @var array<string, string> */
    private const PERMISSIONS = [
        'game_catalog.access' => 'Access Game Catalog administration',
        'game_catalog.snapshots.view' => 'View Game Catalog snapshots and findings',
        'game_catalog.snapshots.import' => 'Import Game Catalog snapshots through approved operator tooling',
        'game_catalog.snapshots.activate' => 'Activate and roll back Game Catalog snapshots',
        'game_catalog.profiles.manage' => 'Manage Game Catalog publication profiles',
        'game_catalog.translations.manage' => 'Manage Game Catalog translations',
        'game_catalog.overrides.manage' => 'Manage reviewed Game Catalog visibility overrides',
    ];

    public function up(): void
    {
        $now = now();
        $platformAdminRoleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');

        foreach (self::PERMISSIONS as $key => $name) {
            DB::table('admin_permissions')->insertOrIgnore([
                'key' => $key,
                'name' => $name,
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            if (! is_int($platformAdminRoleId) && (! is_string($platformAdminRoleId) || ! ctype_digit($platformAdminRoleId))) {
                continue;
            }

            $permissionId = DB::table('admin_permissions')->where('key', $key)->value('id');
            if (is_int($permissionId) || (is_string($permissionId) && ctype_digit($permissionId))) {
                DB::table('admin_role_permissions')->insertOrIgnore([
                    'role_id' => (int) $platformAdminRoleId,
                    'permission_id' => (int) $permissionId,
                ]);
            }
        }
    }

    public function down(): void
    {
        $permissionIds = DB::table('admin_permissions')
            ->whereIn('key', array_keys(self::PERMISSIONS))
            ->pluck('id');

        DB::table('admin_role_permissions')->whereIn('permission_id', $permissionIds)->delete();
        DB::table('admin_permissions')->whereIn('key', array_keys(self::PERMISSIONS))->delete();
    }
};
