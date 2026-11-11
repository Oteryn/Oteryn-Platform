<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    private const PERMISSION_KEY = 'support.content.manage';

    /**
     * @var list<string>
     */
    private const ROLE_KEYS = [
        'content_editor',
        'platform_admin',
    ];

    public function up(): void
    {
        $permissionId = $this->requiredId('admin_permissions', 'key', self::PERMISSION_KEY);

        foreach (self::ROLE_KEYS as $roleKey) {
            DB::table('admin_role_permissions')->insertOrIgnore([
                'role_id' => $this->requiredId('admin_roles', 'key', $roleKey),
                'permission_id' => $permissionId,
            ]);
        }
    }

    public function down(): void
    {
        $permissionId = DB::table('admin_permissions')
            ->where('key', self::PERMISSION_KEY)
            ->value('id');

        if (! is_int($permissionId) && ! (is_string($permissionId) && ctype_digit($permissionId))) {
            return;
        }

        $roleIds = [];

        foreach (DB::table('admin_roles')->whereIn('key', self::ROLE_KEYS)->pluck('id') as $roleId) {
            if (is_int($roleId)) {
                $roleIds[] = $roleId;

                continue;
            }

            if (is_string($roleId) && ctype_digit($roleId)) {
                $roleIds[] = (int) $roleId;
            }
        }

        DB::table('admin_role_permissions')
            ->where('permission_id', (int) $permissionId)
            ->whereIn('role_id', $roleIds)
            ->delete();
    }

    private function requiredId(string $table, string $keyColumn, string $key): int
    {
        $id = DB::table($table)->where($keyColumn, $key)->value('id');

        if (is_int($id)) {
            return $id;
        }

        if (is_string($id) && ctype_digit($id)) {
            return (int) $id;
        }

        throw new RuntimeException("Required RBAC record {$key} is missing.");
    }
};
