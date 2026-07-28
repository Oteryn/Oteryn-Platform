<?php

namespace App\GameCatalog\Http;

use App\GameCatalog\Application\Profiles\CatalogProfileActivator;
use App\GameCatalog\Application\Profiles\CatalogProfileManager;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use Throwable;

final readonly class AdminGameCatalogController
{
    public function __construct(
        private CatalogProfileActivator $activator,
        private CatalogProfileManager $profiles,
    ) {}

    public function index(): View
    {
        $snapshots = DB::table('game_catalog_snapshots as snapshots')
            ->join('game_catalog_releases as content_release', 'content_release.id', '=', 'snapshots.content_target_release_id')
            ->orderByDesc('snapshots.id')
            ->paginate(25, [
                'snapshots.id',
                'snapshots.status',
                'snapshots.schema_version',
                'snapshots.content_sha256',
                'snapshots.generated_at',
                'snapshots.imported_at',
                'snapshots.entity_count',
                'snapshots.relation_count',
                'content_release.display_label as content_release',
            ], 'snapshots_page');

        $profiles = DB::table('game_catalog_profiles as profiles')
            ->join('game_catalog_releases as target', 'target.id', '=', 'profiles.target_release_id')
            ->leftJoin('game_catalog_snapshots as active', 'active.id', '=', 'profiles.active_snapshot_id')
            ->orderBy('profiles.key')
            ->get([
                'profiles.id',
                'profiles.key',
                'profiles.name',
                'profiles.complete_only',
                'profiles.public_enabled',
                'profiles.active_snapshot_id',
                'target.display_label as target_release',
                'active.content_sha256 as active_hash',
            ]);

        $importRuns = DB::table('game_catalog_import_runs')
            ->orderByDesc('id')
            ->limit(50)
            ->get(['id', 'snapshot_id', 'source_name', 'status', 'finding_count', 'started_at', 'finished_at']);

        return view('admin.game-catalog.index', compact('snapshots', 'profiles', 'importRuns'));
    }

    public function snapshot(int $snapshot): View
    {
        $snapshotRow = DB::table('game_catalog_snapshots as snapshots')
            ->join('game_catalog_releases as runtime', 'runtime.id', '=', 'snapshots.runtime_release_id')
            ->join('game_catalog_releases as content', 'content.id', '=', 'snapshots.content_target_release_id')
            ->join('game_catalog_releases as verified', 'verified.id', '=', 'snapshots.verified_content_through_release_id')
            ->where('snapshots.id', $snapshot)
            ->first([
                'snapshots.*',
                'runtime.display_label as runtime_release',
                'content.display_label as content_release',
                'verified.display_label as verified_release',
            ]);
        abort_if($snapshotRow === null, 404);

        $findings = DB::table('game_catalog_validation_findings as findings')
            ->join('game_catalog_import_runs as runs', 'runs.id', '=', 'findings.import_run_id')
            ->where('runs.snapshot_id', $snapshot)
            ->orderByDesc('findings.id')
            ->limit(200)
            ->get(['findings.severity', 'findings.code', 'findings.path', 'findings.message', 'runs.id as import_run_id']);

        $visibility = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as versions', 'versions.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_profiles as profiles', 'profiles.id', '=', 'visibility.profile_id')
            ->where('versions.snapshot_id', $snapshot)
            ->selectRaw('profiles.key as profile_key, visibility.reason_code, visibility.visible, COUNT(*) as total')
            ->groupBy('profiles.key', 'visibility.reason_code', 'visibility.visible')
            ->orderBy('profiles.key')
            ->orderBy('visibility.reason_code')
            ->get();

        return view('admin.game-catalog.snapshot', [
            'snapshot' => $snapshotRow,
            'findings' => $findings,
            'visibility' => $visibility,
        ]);
    }

    public function profile(int $profile): View
    {
        $profileRow = DB::table('game_catalog_profiles as profiles')
            ->join('game_catalog_releases as target', 'target.id', '=', 'profiles.target_release_id')
            ->where('profiles.id', $profile)
            ->first(['profiles.*', 'target.key as target_release_key', 'target.display_label as target_release']);
        abort_if($profileRow === null, 404);

        $releases = DB::table('game_catalog_releases')->orderBy('release_order')->get(['key', 'display_label']);
        $snapshots = DB::table('game_catalog_snapshots as snapshots')
            ->join('game_catalog_releases as content', 'content.id', '=', 'snapshots.content_target_release_id')
            ->where('snapshots.status', 'validated')
            ->orderByDesc('snapshots.id')
            ->get(['snapshots.id', 'snapshots.content_sha256', 'snapshots.generated_at', 'content.display_label as content_release']);

        $entityReasons = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as versions', 'versions.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'versions.entity_id')
            ->where('visibility.profile_id', $profile)
            ->selectRaw('visibility.reason_code, visibility.visible, COUNT(*) as total')
            ->groupBy('visibility.reason_code', 'visibility.visible')
            ->orderBy('visibility.reason_code')
            ->get();
        $relationReasons = DB::table('game_catalog_profile_relations')
            ->where('profile_id', $profile)
            ->selectRaw('reason_code, visible, COUNT(*) as total')
            ->groupBy('reason_code', 'visible')
            ->orderBy('reason_code')
            ->get();
        $hiddenEntities = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as versions', 'versions.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'versions.entity_id')
            ->where('visibility.profile_id', $profile)
            ->where('visibility.visible', false)
            ->orderBy('entities.entity_type')
            ->orderBy('entities.canonical_key')
            ->limit(200)
            ->get(['entities.entity_type', 'entities.canonical_key', 'visibility.reason_code']);
        $history = DB::table('game_catalog_activation_history')
            ->where('profile_id', $profile)
            ->orderByDesc('id')
            ->limit(50)
            ->get();

        return view('admin.game-catalog.profile', compact(
            'profileRow',
            'releases',
            'snapshots',
            'entityReasons',
            'relationReasons',
            'hiddenEntities',
            'history',
        ));
    }

    public function activate(Request $request, int $profile): RedirectResponse
    {
        $validated = $request->validate([
            'snapshot_id' => ['required', 'integer', 'min:1'],
            'action' => ['required', 'in:activate,rollback'],
            'reason' => ['nullable', 'string', 'max:500'],
        ]);
        $profileRow = DB::table('game_catalog_profiles')->whereKey($profile)->first(['key']);
        abort_if($profileRow === null, 404);

        try {
            $this->activator->activate(
                (int) $validated['snapshot_id'],
                (string) $profileRow->key,
                $this->actorId($request),
                $validated['action'],
                $validated['reason'] ?? null,
            );
        } catch (Throwable $exception) {
            return back()->withErrors(['catalog' => $exception->getMessage()]);
        }

        return back()->with('status', $validated['action'] === 'rollback' ? 'Snapshot rollback completed.' : 'Snapshot activation completed.');
    }

    public function updateProfile(Request $request, int $profile): RedirectResponse
    {
        $validated = $request->validate([
            'target_release' => ['required', 'string', 'max:32'],
            'complete_only' => ['nullable', 'boolean'],
            'public_enabled' => ['nullable', 'boolean'],
        ]);

        try {
            $this->profiles->update(
                $profile,
                $validated['target_release'],
                (bool) ($validated['complete_only'] ?? false),
                (bool) ($validated['public_enabled'] ?? false),
                $this->actorId($request),
            );
        } catch (Throwable $exception) {
            return back()->withInput()->withErrors(['catalog' => $exception->getMessage()]);
        }

        return back()->with('status', 'Game Catalog profile updated and visibility recomputed.');
    }

    private function actorId(Request $request): ?int
    {
        $actor = $request->user();

        return $actor instanceof Identity ? $actor->id : null;
    }
}
