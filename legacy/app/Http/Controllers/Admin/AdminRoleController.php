<?php

namespace App\Http\Controllers\Admin;

use App\Admin\AdminRoleManager;
use App\Http\Requests\Admin\AdminRoleAssignmentRequest;
use App\Identity\Models\Identity;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final class AdminRoleController
{
    public function index(): View
    {
        $identities = Identity::query()
            ->orderBy('email')
            ->orderBy('id')
            ->paginate(50);

        $identityIds = $identities->getCollection()->pluck('id')->all();
        $rolesByIdentity = DB::table('identity_admin_roles')
            ->join('admin_roles', 'admin_roles.id', '=', 'identity_admin_roles.role_id')
            ->whereIn('identity_admin_roles.identity_id', $identityIds)
            ->orderBy('admin_roles.key')
            ->get(['identity_admin_roles.identity_id', 'admin_roles.key'])
            ->groupBy('identity_id');

        return view('admin.roles.index', [
            'identities' => $identities,
            'rolesByIdentity' => $rolesByIdentity,
            'availableRoles' => AdminRoleManager::roles(),
        ]);
    }

    public function store(
        AdminRoleAssignmentRequest $request,
        Identity $identity,
        AdminRoleManager $roles,
    ): RedirectResponse {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        try {
            $roles->assign($actor, $identity->id, $request->string('role')->toString());
        } catch (DomainException|InvalidArgumentException $exception) {
            return back()->withErrors(['role' => $exception->getMessage()]);
        }

        return redirect()->route('admin.roles.index')->with('status', 'Administrator role assignment updated.');
    }

    public function destroy(
        Request $request,
        Identity $identity,
        string $roleKey,
        AdminRoleManager $roles,
    ): RedirectResponse {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        try {
            $roles->remove($actor, $identity->id, $roleKey);
        } catch (DomainException|InvalidArgumentException $exception) {
            return back()->withErrors(['role' => $exception->getMessage()]);
        }

        return redirect()->route('admin.roles.index')->with('status', 'Administrator role assignment updated.');
    }
}
