<?php

namespace App\Http\Controllers\Downloads;

use App\Downloads\Actions\PublishClientRelease;
use App\Downloads\Actions\SaveClientRelease;
use App\Downloads\Models\ClientRelease;
use App\Http\Requests\Downloads\PublishClientReleaseRequest;
use App\Http\Requests\Downloads\SaveClientReleaseRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class AdminDownloadController
{
    public function index(): View
    {
        return view('admin.downloads.index', [
            'releases' => ClientRelease::query()
                ->withCount('artifacts')
                ->orderByDesc('updated_at')
                ->orderByDesc('id')
                ->paginate(25),
        ]);
    }

    public function create(): View
    {
        return view('admin.downloads.form', ['release' => null]);
    }

    public function store(SaveClientReleaseRequest $request, SaveClientRelease $save): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);
        $validated = $request->validated();
        $releaseNotes = $validated['release_notes'] ?? null;

        $release = $save->execute(
            $identity,
            null,
            $request->string('version')->toString(),
            $request->string('channel')->toString(),
            is_string($releaseNotes) ? $releaseNotes : null,
            $request->artifactInput(),
        );

        return redirect()
            ->route('admin.downloads.edit', $release)
            ->with('status', 'Client release draft saved.');
    }

    public function edit(ClientRelease $clientRelease): View
    {
        return view('admin.downloads.form', [
            'release' => $clientRelease->load('artifacts'),
        ]);
    }

    public function update(
        SaveClientReleaseRequest $request,
        ClientRelease $clientRelease,
        SaveClientRelease $save,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);
        $validated = $request->validated();
        $releaseNotes = $validated['release_notes'] ?? null;

        $save->execute(
            $identity,
            $clientRelease,
            $request->string('version')->toString(),
            $request->string('channel')->toString(),
            is_string($releaseNotes) ? $releaseNotes : null,
            $request->artifactInput(),
        );

        return redirect()
            ->route('admin.downloads.edit', $clientRelease)
            ->with('status', 'Client release draft saved.');
    }

    public function publish(
        PublishClientReleaseRequest $request,
        ClientRelease $clientRelease,
        PublishClientRelease $publish,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $published = $publish->execute($identity, $clientRelease, $request->makeCurrent());

        return redirect()
            ->route('admin.downloads.edit', $published)
            ->with('status', $published->is_current
                ? 'Client release published and set as current.'
                : 'Client release published.');
    }
}
