<?php

namespace App\Admin;

use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;

final class AdminAuthorization
{
    /**
     * Read-only navigation projection; route middleware still authorizes each request.
     * No cross-request cache: role revocation is reflected on the next render.
     *
     * @return list<string>
     */
    public function grantedPermissions(Identity $identity): array
    {
        $permissions = DB::table('identity_admin_roles')
            ->join('admin_role_permissions', 'admin_role_permissions.role_id', '=', 'identity_admin_roles.role_id')
            ->join('admin_permissions', 'admin_permissions.id', '=', 'admin_role_permissions.permission_id')
            ->where('identity_admin_roles.identity_id', $identity->id)
            ->whereIn('admin_permissions.key', AdminPermission::all())
            ->distinct()
            ->orderBy('admin_permissions.key')
            ->pluck('admin_permissions.key')
            ->all();

        return array_values(array_filter($permissions, static fn (mixed $permission): bool => is_string($permission)));
    }

    public function allows(Identity $identity, string $permission): bool
    {
        if (! in_array($permission, AdminPermission::all(), true)) {
            return false;
        }

        return DB::table('identity_admin_roles')
            ->join('admin_role_permissions', 'admin_role_permissions.role_id', '=', 'identity_admin_roles.role_id')
            ->join('admin_permissions', 'admin_permissions.id', '=', 'admin_role_permissions.permission_id')
            ->where('identity_admin_roles.identity_id', $identity->id)
            ->where('admin_permissions.key', $permission)
            ->exists();
    }
}
