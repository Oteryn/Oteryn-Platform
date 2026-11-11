<?php

namespace App\Support\Http;

use App\Identity\Models\Identity;
use App\Support\Actions\ManagePlayerReport;
use App\Support\Http\Requests\AdminReportModerationRequest;
use App\Support\Models\PlayerReport;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class AdminPlayerReportController
{
    public function index(Request $request): View
    {
        $status = $request->query('status');
        $query = PlayerReport::query()->with('reporter')->orderByDesc('created_at');
        if (is_string($status) && in_array($status, PlayerReport::statuses(), true)) {
            $query->where('status', $status);
        }

        return view('admin.moderation.reports.index', [
            'reports' => $query->paginate(30)->withQueryString(),
            'statuses' => PlayerReport::statuses(),
            'selectedStatus' => is_string($status) ? $status : null,
        ]);
    }

    public function show(PlayerReport $playerReport): View
    {
        return view('admin.moderation.reports.show', [
            'report' => $playerReport->load(['reporter', 'assignee']),
            'statuses' => PlayerReport::statuses(),
        ]);
    }

    public function update(
        AdminReportModerationRequest $request,
        PlayerReport $playerReport,
        ManagePlayerReport $reports,
    ): RedirectResponse {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        try {
            $reports->moderate(
                $actor,
                $playerReport,
                $request->string('status')->toString(),
                $request->filled('public_outcome') ? $request->string('public_outcome')->toString() : null,
                $request->filled('moderator_notes') ? $request->string('moderator_notes')->toString() : null,
                $request->integer('lock_version'),
                app()->getLocale(),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('admin.moderation.reports.show', $playerReport)->with('status', __('support.status.report_updated'));
    }
}
