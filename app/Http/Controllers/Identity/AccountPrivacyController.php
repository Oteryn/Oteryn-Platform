<?php

namespace App\Http\Controllers\Identity;

use App\Audit\SecurityEventRecorder;
use App\Http\Requests\Identity\UpdateAccountPrivacyRequest;
use App\Identity\Models\Identity;
use Illuminate\Http\RedirectResponse;
use Illuminate\Support\Facades\DB;

final class AccountPrivacyController
{
    public function update(
        UpdateAccountPrivacyRequest $request,
        SecurityEventRecorder $securityEvents,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        DB::transaction(function () use ($identity, $request, $securityEvents): void {
            $locked = Identity::query()->lockForUpdate()->find($identity->id);
            abort_unless($locked instanceof Identity && $locked->terminated_at === null, 403);

            $locked->forceFill($request->privacy())->save();
            $securityEvents->recordIdentityPrivacyUpdated($locked->id);
        });

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', __('identity.status.privacy_updated'));
    }
}
