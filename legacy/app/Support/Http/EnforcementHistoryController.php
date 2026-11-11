<?php

namespace App\Support\Http;

use App\Identity\Models\Identity;
use App\Support\Actions\ManageEnforcementRecord;
use App\Support\Http\Requests\EnforcementAppealRequest;
use App\Support\Models\EnforcementRecord;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class EnforcementHistoryController
{
    public function index(Request $request): View
    {
        $identity = $this->identity($request);

        return view('support.enforcement.index', [
            'records' => EnforcementRecord::query()
                ->where('identity_id', $identity->id)
                ->orderByDesc('effective_at')
                ->paginate(20),
        ]);
    }

    public function show(Request $request, EnforcementRecord $enforcementRecord): View
    {
        $identity = $this->identity($request);
        $this->assertOwner($identity, $enforcementRecord);

        return view('support.enforcement.show', [
            'record' => $enforcementRecord,
        ]);
    }

    public function acknowledge(
        Request $request,
        EnforcementRecord $enforcementRecord,
        ManageEnforcementRecord $records,
    ): RedirectResponse {
        $identity = $this->identity($request);
        $request->validate(['lock_version' => ['required', 'integer', 'min:1']]);

        try {
            $records->acknowledge($identity, $enforcementRecord, $request->integer('lock_version'));
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('support.enforcement.show', $enforcementRecord)->with('status', __('support.status.enforcement_acknowledged'));
    }

    public function appeal(
        EnforcementAppealRequest $request,
        EnforcementRecord $enforcementRecord,
        ManageEnforcementRecord $records,
    ): RedirectResponse {
        $identity = $this->identity($request);

        try {
            $records->appeal(
                $identity,
                $enforcementRecord,
                $request->string('appeal_message')->toString(),
                $request->integer('lock_version'),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('support.enforcement.show', $enforcementRecord)->with('status', __('support.status.appeal_submitted'));
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }

    private function assertOwner(Identity $identity, EnforcementRecord $record): void
    {
        abort_unless($record->identity_id === $identity->id, 404);
    }
}
