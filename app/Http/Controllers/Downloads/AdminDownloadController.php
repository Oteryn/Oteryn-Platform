<?php

namespace App\Http\Controllers\Downloads;

use App\Downloads\Actions\ActivateUpdaterGeneration;
use App\Downloads\Actions\ApproveUpdaterPolicy;
use App\Downloads\Actions\EnableUpdaterRelease;
use App\Downloads\Actions\ImportSignedUpdaterGeneration;
use App\Downloads\Actions\PublishClientRelease;
use App\Downloads\Actions\SaveClientRelease;
use App\Downloads\Actions\WithdrawUpdaterRelease;
use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
use App\Downloads\Updater\UpdaterPolicyDocument;
use App\Http\Requests\Downloads\ApproveUpdaterPolicyRequest;
use App\Http\Requests\Downloads\ImportUpdaterGenerationRequest;
use App\Http\Requests\Downloads\PublishClientReleaseRequest;
use App\Http\Requests\Downloads\SaveClientReleaseRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

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
        $validated = $request->validated();
        $releaseNotes = $validated['release_notes'] ?? null;

        $release = $save->execute(
            $this->identity($request),
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
        $validated = $request->validated();
        $releaseNotes = $validated['release_notes'] ?? null;

        $save->execute(
            $this->identity($request),
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
        $published = $publish->execute(
            $this->identity($request),
            $clientRelease,
            $request->makeCurrent(),
        );

        return redirect()
            ->route('admin.downloads.edit', $published)
            ->with('status', $published->is_current
                ? 'Client release published and set as current.'
                : 'Client release published.');
    }

    public function enableUpdater(
        Request $request,
        ClientRelease $clientRelease,
        EnableUpdaterRelease $enable,
    ): RedirectResponse {
        $release = $enable->execute($this->identity($request), $clientRelease);

        return redirect()
            ->route('admin.downloads.edit', $release)
            ->with('status', 'Updater release identity enabled without changing browser publication state.');
    }

    public function withdrawUpdater(
        Request $request,
        ClientRelease $clientRelease,
        WithdrawUpdaterRelease $withdraw,
    ): RedirectResponse {
        $release = $withdraw->execute($this->identity($request), $clientRelease);

        return redirect()
            ->route('admin.downloads.edit', $release)
            ->with('status', 'Updater release withdrawn from future selection; immutable release history was preserved.');
    }

    public function updater(string $channel, UpdaterPolicyDocument $documents): View
    {
        $this->assertChannel($channel);
        $releases = ClientRelease::query()
            ->where('channel', $channel)
            ->whereNotNull('updater_release_id')
            ->with('artifacts')
            ->orderBy('updater_sequence')
            ->get();
        $policies = ClientUpdatePolicy::query()
            ->where('channel', $channel)
            ->with('currentRelease')
            ->orderByDesc('revision')
            ->limit(25)
            ->get();
        $generations = ClientUpdateGeneration::query()
            ->where('channel', $channel)
            ->with('policy')
            ->orderByDesc('timestamp_version')
            ->limit(25)
            ->get();
        $policyDocuments = [];
        foreach ($policies as $policy) {
            $policyDocuments[$policy->id] = $documents->encodePolicy($policy);
        }

        return view('admin.downloads.updater', [
            'channel' => $channel,
            'releases' => $releases,
            'policies' => $policies,
            'generations' => $generations,
            'policyDocuments' => $policyDocuments,
            'operationId' => Str::uuid()->toString(),
        ]);
    }

    public function approveUpdaterPolicy(
        ApproveUpdaterPolicyRequest $request,
        string $channel,
        ApproveUpdaterPolicy $approve,
    ): RedirectResponse {
        $this->assertChannel($channel);
        $input = $request->policyInput();

        $policy = $approve->execute(
            $this->identity($request),
            $input['operation_id'],
            $channel,
            $input['current_release_id'],
            $input['minimum_supported_release_sequence'],
            $input['update_mode'],
            $input['rollback_authorization'],
            $input['revoked_release_ids'],
            $input['revoked_artifact_targets'],
        );

        return redirect()
            ->route('admin.downloads.updater', ['channel' => $channel])
            ->with('status', "Updater policy revision {$policy->revision} approved for signing/repository reconciliation.");
    }

    public function importUpdaterGeneration(
        ImportUpdaterGenerationRequest $request,
        string $channel,
        ImportSignedUpdaterGeneration $import,
    ): RedirectResponse {
        $this->assertChannel($channel);
        $generation = $import->execute($this->identity($request), $request->generationPayload());

        return redirect()
            ->route('admin.downloads.updater', ['channel' => $channel])
            ->with('status', "Public signed-generation metadata {$generation->generation_id} reconciled. No signing key was used or stored.");
    }

    public function activateUpdaterGeneration(
        Request $request,
        string $channel,
        ClientUpdateGeneration $clientUpdateGeneration,
        ActivateUpdaterGeneration $activate,
    ): RedirectResponse {
        $this->assertChannel($channel);
        abort_unless($clientUpdateGeneration->channel === $channel, 404);
        $generation = $activate->execute($this->identity($request), $clientUpdateGeneration);

        return redirect()
            ->route('admin.downloads.updater', ['channel' => $channel])
            ->with('status', "Generation {$generation->generation_id} is Platform-active for updater state. This is not production deployment or client trust verification.");
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }

    private function assertChannel(string $channel): void
    {
        abort_unless(in_array($channel, DownloadCatalog::channels(), true), 404);
    }
}