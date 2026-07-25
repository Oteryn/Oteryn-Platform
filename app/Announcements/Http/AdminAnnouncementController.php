<?php

namespace App\Announcements\Http;

use App\Announcements\Actions\SaveAnnouncement;
use App\Announcements\Models\SiteAnnouncement;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class AdminAnnouncementController
{
    public function index(): View
    {
        return view('admin.announcements.index', [
            'announcements' => SiteAnnouncement::query()
                ->orderByDesc('updated_at')
                ->orderByDesc('id')
                ->paginate(25),
        ]);
    }

    public function create(): View
    {
        return view('admin.announcements.form', [
            'announcement' => null,
        ]);
    }

    public function store(AnnouncementRequest $request, SaveAnnouncement $save): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $announcement = $save->execute(
            $identity,
            null,
            $request->string('title')->toString(),
            $request->string('body')->toString(),
            $request->string('severity')->toString(),
            $this->utc($request->string('starts_at')->toString()),
            $request->filled('ends_at') ? $this->utc($request->string('ends_at')->toString()) : null,
            $request->string('publication_state')->toString(),
            $request->filled('action_label') ? $request->string('action_label')->toString() : null,
            $request->filled('action_url') ? $request->string('action_url')->toString() : null,
            null,
        );

        return redirect()
            ->route('admin.announcements.edit', $announcement)
            ->with('status', 'Announcement saved.');
    }

    public function edit(SiteAnnouncement $siteAnnouncement): View
    {
        return view('admin.announcements.form', [
            'announcement' => $siteAnnouncement,
        ]);
    }

    public function update(
        AnnouncementRequest $request,
        SiteAnnouncement $siteAnnouncement,
        SaveAnnouncement $save,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $announcement = $save->execute(
                $identity,
                $siteAnnouncement,
                $request->string('title')->toString(),
                $request->string('body')->toString(),
                $request->string('severity')->toString(),
                $this->utc($request->string('starts_at')->toString()),
                $request->filled('ends_at') ? $this->utc($request->string('ends_at')->toString()) : null,
                $request->string('publication_state')->toString(),
                $request->filled('action_label') ? $request->string('action_label')->toString() : null,
                $request->filled('action_url') ? $request->string('action_url')->toString() : null,
                $request->integer('lock_version'),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()
            ->route('admin.announcements.edit', $announcement)
            ->with('status', 'Announcement saved.');
    }

    private function utc(string $value): CarbonImmutable
    {
        $date = CarbonImmutable::createFromFormat('!Y-m-d\TH:i', $value, 'UTC');

        if ($date === null) {
            abort(422, 'The date must be a valid UTC date and time.');
        }

        return $date;
    }
}
