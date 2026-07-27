<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    /**
     * @var array<string, list<string>>
     */
    private const ROLE_PERMISSIONS = [
        'content_editor' => [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.categories.manage',
        ],
        'platform_admin' => [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.categories.manage',
            'wiki.publish',
        ],
    ];

    public function up(): void
    {
        DB::transaction(function (): void {
            foreach (self::ROLE_PERMISSIONS as $roleKey => $permissionKeys) {
                $roleId = $this->requiredId('admin_roles', 'key', $roleKey);

                foreach ($permissionKeys as $permissionKey) {
                    DB::table('admin_role_permissions')->insertOrIgnore([
                        'role_id' => $roleId,
                        'permission_id' => $this->requiredId('admin_permissions', 'key', $permissionKey),
                    ]);
                }
            }
        }, 3);
    }

    public function down(): void
    {
        DB::transaction(function (): void {
            foreach (self::ROLE_PERMISSIONS as $roleKey => $permissionKeys) {
                $roleId = $this->optionalId('admin_roles', 'key', $roleKey);

                if ($roleId === null) {
                    continue;
                }

                $permissionIds = DB::table('admin_permissions')
                    ->whereIn('key', $permissionKeys)
                    ->pluck('id')
                    ->map(fn (mixed $id): ?int => $this->integerId($id))
                    ->filter(fn (?int $id): bool => $id !== null)
                    ->values()
                    ->all();

                if ($permissionIds === []) {
                    continue;
                }

                DB::table('admin_role_permissions')
                    ->where('role_id', $roleId)
                    ->whereIn('permission_id', $permissionIds)
                    ->delete();
            }
        }, 3);
    }

    private function requiredId(string $table, string $keyColumn, string $key): int
    {
        $id = $this->optionalId($table, $keyColumn, $key);

        if ($id !== null) {
            return $id;
        }

        throw new RuntimeException("Required RBAC record {$key} is missing.");
    }

    private function optionalId(string $table, string $keyColumn, string $key): ?int
    {
        return $this->integerId(DB::table($table)->where($keyColumn, $key)->value('id'));
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
