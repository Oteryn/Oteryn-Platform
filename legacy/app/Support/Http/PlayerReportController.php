<?php

namespace App\Support\Http;

use App\Identity\Models\Identity;
use App\Support\Actions\ManagePlayerReport;
use App\Support\Http\Requests\PlayerReportRequest;
use App\Support\Models\PlayerReport;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

final class PlayerReportController
{
    public function index(Request $request): View
    {
        $identity = $this->identity($request);

        return view('support.reports.index', [
            'reports' => PlayerReport::query()
                ->where('reporter_identity_id', $identity->id)
                ->orderByDesc('created_at')
                ->paginate(20),
        ]);
    }

    public function create(Request $request): View
    {
        $this->identity($request);

        return view('support.reports.create', [
            'types' => PlayerReport::types(),
            'categories' => ManagePlayerReport::categories(),
            'requestKey' => (string) Str::uuid(),
            'attachmentsEnabled' => (bool) config('support.attachments.enabled', false),
        ]);
    }

    public function store(PlayerReportRequest $request, ManagePlayerReport $reports): RedirectResponse
    {
        $identity = $this->identity($request);

        try {
            $report = $reports->submit(
                $identity,
                $request->string('request_key')->toString(),
                $request->string('report_type')->toString(),
                $request->string('category')->toString(),
                $request->string('target_reference')->toString(),
                $request->filled('evidence_summary') ? $request->string('evidence_summary')->toString() : null,
            );
        } catch (DomainException $exception) {
            return back()->withInput()->withErrors(['report' => $exception->getMessage()]);
        }

        return redirect()->route('support.reports.show', $report)->with('status', __('support.status.report_created'));
    }

    public function show(Request $request, PlayerReport $playerReport): View
    {
        $identity = $this->identity($request);
        abort_unless($playerReport->reporter_identity_id === $identity->id, 404);

        return view('support.reports.show', [
            'report' => $playerReport,
        ]);
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }
}
