<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    private const PERMISSION = 'media.manage';

    public function up(): void
    {
        DB::transaction(function (): void {
            $contentEditorRoleId = DB::table('admin_roles')->where('key', 'content_editor')->value('id');
            $platformAdminRoleId = DB::table('admin_roles')->where('key', 'platform_admin')->value('id');

            if (! $this->isIntegerCompatible($contentEditorRoleId) || ! $this->isIntegerCompatible($platformAdminRoleId)) {
                throw new RuntimeException('Required administrator roles do not exist.');
            }

            $now = now();
            $permissionId = DB::table('admin_permissions')->insertGetId([
                'key' => self::PERMISSION,
                'name' => 'Manage the editorial image library',
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            DB::table('admin_role_permissions')->insert([
                [
                    'role_id' => (int) $contentEditorRoleId,
                    'permission_id' => $permissionId,
                ],
                [
                    'role_id' => (int) $platformAdminRoleId,
                    'permission_id' => $permissionId,
                ],
            ]);
        }, 3);
    }

    public function down(): void
    {
        DB::transaction(function (): void {
            $permissionId = DB::table('admin_permissions')->where('key', self::PERMISSION)->value('id');

            if ($this->isIntegerCompatible($permissionId)) {
                DB::table('admin_role_permissions')->where('permission_id', (int) $permissionId)->delete();
            }

            DB::table('admin_permissions')->where('key', self::PERMISSION)->delete();
        }, 3);
    }

    private function isIntegerCompatible(mixed $value): bool
    {
        return is_int($value) || (is_string($value) && ctype_digit($value));
    }
};
