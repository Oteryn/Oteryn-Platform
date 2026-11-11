<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    private const PERMISSION = 'media.manage';

    public function up(): void
    {
        DB::transaction(function (): void {
            $contentEditorRoleId = $this->integerId(
                DB::table('admin_roles')->where('key', 'content_editor')->value('id'),
            );
            $platformAdminRoleId = $this->integerId(
                DB::table('admin_roles')->where('key', 'platform_admin')->value('id'),
            );

            if ($contentEditorRoleId === null || $platformAdminRoleId === null) {
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
                    'role_id' => $contentEditorRoleId,
                    'permission_id' => $permissionId,
                ],
                [
                    'role_id' => $platformAdminRoleId,
                    'permission_id' => $permissionId,
                ],
            ]);
        }, 3);
    }

    public function down(): void
    {
        DB::transaction(function (): void {
            $permissionId = $this->integerId(
                DB::table('admin_permissions')->where('key', self::PERMISSION)->value('id'),
            );

            if ($permissionId !== null) {
                DB::table('admin_role_permissions')->where('permission_id', $permissionId)->delete();
            }

            DB::table('admin_permissions')->where('key', self::PERMISSION)->delete();
        }, 3);
    }

    private function integerId(mixed $value): ?int
    {
        if (is_int($value)) {
            return $value;
        }

        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        return null;
    }
};
