<?php

namespace App\Support\Http;

use App\Identity\Models\Identity;
use App\Support\Actions\ManageEnforcementRecord;
use App\Support\Http\Requests\AdminEnforcementAppealRequest;
use App\Support\Http\Requests\EnforcementRecordRequest;
use App\Support\Models\EnforcementRecord;
use Carbon\CarbonImmutable;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class AdminEnforcementController
{
    public function index(Request $request): View
    {
        $status = $request->query('status');
        $query = EnforcementRecord::query()->with('identity')->orderByDesc('effective_at');
        if (is_string($status) && in_array($status, EnforcementRecord::statuses(), true)) {
            $query->where('status', $status);
        }

        return view('admin.moderation.enforcement.index', [
            'records' => $query->paginate(30)->withQueryString(),
            'statuses' => EnforcementRecord::statuses(),
            'selectedStatus' => is_string($status) ? $status : null,
        ]);
    }

    public function create(): View
    {
        return $this->form(null);
    }

    public function store(
        EnforcementRecordRequest $request,
        ManageEnforcementRecord $records,
    ): RedirectResponse {
        $actor = $this->identity($request);
        $record = $records->create(
            $actor,
            $request->integer('target_identity_id'),
            $request->string('category')->toString(),
            $request->string('status')->toString(),
            $request->string('public_reason')->toString(),
            $request->filled('moderator_notes') ? $request->string('moderator_notes')->toString() : null,
            CarbonImmutable::parse($request->string('effective_at')->toString(), 'UTC'),
            $request->filled('expires_at')
                ? CarbonImmutable::parse($request->string('expires_at')->toString(), 'UTC')
                : null,
            app()->getLocale(),
        );

        return redirect()->route('admin.moderation.enforcement.show', $record)->with('status', __('support.status.enforcement_created'));
    }

    public function show(EnforcementRecord $enforcementRecord): View
    {
        return view('admin.moderation.enforcement.show', [
            'record' => $enforcementRecord->load('identity'),
        ]);
    }

    public function edit(EnforcementRecord $enforcementRecord): View
    {
        return $this->form($enforcementRecord->load('identity'));
    }

    public function update(
        EnforcementRecordRequest $request,
        EnforcementRecord $enforcementRecord,
        ManageEnforcementRecord $records,
    ): RedirectResponse {
        $actor = $this->identity($request);

        try {
            $records->update(
                $actor,
                $enforcementRecord,
                $request->string('category')->toString(),
                $request->string('status')->toString(),
                $request->string('public_reason')->toString(),
                $request->filled('moderator_notes') ? $request->string('moderator_notes')->toString() : null,
                CarbonImmutable::parse($request->string('effective_at')->toString(), 'UTC'),
                $request->filled('expires_at')
                    ? CarbonImmutable::parse($request->string('expires_at')->toString(), 'UTC')
                    : null,
                $request->integer('lock_version'),
                app()->getLocale(),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('admin.moderation.enforcement.show', $enforcementRecord)->with('status', __('support.status.enforcement_updated'));
    }

    public function appeal(
        AdminEnforcementAppealRequest $request,
        EnforcementRecord $enforcementRecord,
        ManageEnforcementRecord $records,
    ): RedirectResponse {
        $actor = $this->identity($request);

        try {
            $records->resolveAppeal(
                $actor,
                $enforcementRecord,
                $request->string('appeal_status')->toString(),
                $request->filled('appeal_outcome') ? $request->string('appeal_outcome')->toString() : '',
                $request->integer('lock_version'),
                app()->getLocale(),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('admin.moderation.enforcement.show', $enforcementRecord)->with('status', __('support.status.appeal_updated'));
    }

    private function form(?EnforcementRecord $record): View
    {
        return view('admin.moderation.enforcement.form', [
            'record' => $record,
            'categories' => EnforcementRecord::categories(),
            'statuses' => EnforcementRecord::statuses(),
        ]);
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }
}
