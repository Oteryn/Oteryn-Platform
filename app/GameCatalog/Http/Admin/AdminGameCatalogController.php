<?php

namespace App\GameCatalog\Http\Admin;

use App\GameCatalog\Application\Diff\CatalogSnapshotDiff;
use App\GameCatalog\Application\Diff\CatalogSnapshotDiffService;
use Illuminate\Contracts\View\View;
use Illuminate\Database\Query\Builder;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use RuntimeException;

final class AdminGameCatalogController
{
    public function index(): View
    {
        return view('game-catalog.admin.index', [
            'snapshotCount' => DB::table('game_catalog_snapshots')->count(),
            'profileCount' => DB::table('game_catalog_profiles')->count(),
            'findingCount' => DB::table('game_catalog_validation_findings')->count(),
            'snapshots' => $this->snapshotQuery()->limit(50)->get(),
            'profiles' => $this->profileQuery()->limit(50)->get(),
        ]);
    }

    public function snapshots(): View
    {
        return view('game-catalog.admin.snapshots', [
            'snapshots' => $this->snapshotQuery()->limit(100)->get(),
        ]);
    }

    public function snapshot(int $snapshot): View
    {
        $record = $this->snapshotQuery()->where('snapshots.id', $snapshot)->first();
        if ($record === null) {
            abort(404);
        }

        return view('game-catalog.admin.snapshot', [
            'snapshot' => $record,
            'entities' => DB::table('game_catalog_entity_snapshots as entity_snapshots')
                ->join('game_catalog_entities as entities', 'entities.id', '=', 'entity_snapshots.entity_id')
                ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'entity_snapshots.introduced_release_id')
                ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'entity_snapshots.removed_release_id')
                ->where('entity_snapshots.snapshot_id', $snapshot)
                ->orderBy('entities.entity_type')
                ->orderBy('entities.canonical_key')
                ->limit(200)
                ->get([
                    'entities.canonical_key',
                    'entities.entity_type',
                    'entity_snapshots.completeness',
                    'entity_snapshots.availability',
                    'entity_snapshots.runtime_present',
                    'entity_snapshots.enabled',
                    'entity_snapshots.data_sha256',
                    'introduced.display_label as introduced_release',
                    'removed.display_label as removed_release',
                ]),
            'relations' => DB::table('game_catalog_relation_snapshots as relations')
                ->join('game_catalog_entities as source', 'source.id', '=', 'relations.source_entity_id')
                ->leftJoin('game_catalog_entities as target', 'target.id', '=', 'relations.target_entity_id')
                ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'relations.introduced_release_id')
                ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'relations.removed_release_id')
                ->where('relations.snapshot_id', $snapshot)
                ->orderBy('relations.relation_type')
                ->orderBy('relations.canonical_key')
                ->limit(200)
                ->get([
                    'relations.canonical_key',
                    'relations.relation_type',
                    'relations.completeness',
                    'relations.enabled',
                    'relations.data_sha256',
                    'source.canonical_key as source_key',
                    'target.canonical_key as target_key',
                    'introduced.display_label as introduced_release',
                    'removed.display_label as removed_release',
                ]),
            'visibility' => DB::table('game_catalog_profile_entities as projection')
                ->join('game_catalog_profiles as profiles', 'profiles.id', '=', 'projection.profile_id')
                ->join('game_catalog_entity_snapshots as entity_snapshots', 'entity_snapshots.id', '=', 'projection.entity_snapshot_id')
                ->where('entity_snapshots.snapshot_id', $snapshot)
                ->groupBy('profiles.id', 'profiles.key', 'profiles.name', 'projection.visible', 'projection.reason_code')
                ->orderBy('profiles.key')
                ->orderBy('projection.visible', 'desc')
                ->orderBy('projection.reason_code')
                ->get([
                    'profiles.id as profile_id',
                    'profiles.key as profile_key',
                    'profiles.name as profile_name',
                    'projection.visible',
                    'projection.reason_code',
                    DB::raw('COUNT(*) as record_count'),
                ]),
            'findings' => DB::table('game_catalog_validation_findings')
                ->where('snapshot_id', $snapshot)
                ->orderByDesc('id')
                ->limit(100)
                ->get(),
            'auditEvents' => DB::table('game_catalog_audit_events')
                ->where('snapshot_id', $snapshot)
                ->orderByDesc('id')
                ->limit(100)
                ->get(),
        ]);
    }

    public function profiles(): View
    {
        return view('game-catalog.admin.profiles', [
            'profiles' => $this->profileQuery()->limit(100)->get(),
        ]);
    }

    public function profile(int $profile): View
    {
        $record = $this->profileQuery()->where('profiles.id', $profile)->first();
        if ($record === null) {
            abort(404);
        }

        return view('game-catalog.admin.profile', [
            'profile' => $record,
            'entityVisibility' => DB::table('game_catalog_profile_entities as projection')
                ->join('game_catalog_entity_snapshots as entity_snapshots', 'entity_snapshots.id', '=', 'projection.entity_snapshot_id')
                ->join('game_catalog_entities as entities', 'entities.id', '=', 'entity_snapshots.entity_id')
                ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'entity_snapshots.introduced_release_id')
                ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'entity_snapshots.removed_release_id')
                ->where('projection.profile_id', $profile)
                ->orderBy('projection.visible', 'desc')
                ->orderBy('projection.reason_code')
                ->orderBy('entities.canonical_key')
                ->limit(200)
                ->get([
                    'entities.canonical_key',
                    'entities.entity_type',
                    'entity_snapshots.completeness',
                    'entity_snapshots.availability',
                    'projection.visible',
                    'projection.reason_code',
                    'projection.computed_at',
                    'introduced.display_label as introduced_release',
                    'removed.display_label as removed_release',
                ]),
            'relationVisibility' => DB::table('game_catalog_profile_relations as projection')
                ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'projection.relation_snapshot_id')
                ->where('projection.profile_id', $profile)
                ->orderBy('projection.visible', 'desc')
                ->orderBy('projection.reason_code')
                ->orderBy('relations.canonical_key')
                ->limit(200)
                ->get([
                    'relations.canonical_key',
                    'relations.relation_type',
                    'relations.completeness',
                    'projection.visible',
                    'projection.reason_code',
                    'projection.computed_at',
                ]),
            'auditEvents' => DB::table('game_catalog_audit_events')
                ->where('profile_id', $profile)
                ->orderByDesc('id')
                ->limit(100)
                ->get(),
        ]);
    }

    public function findings(Request $request): View
    {
        /** @var array{severity?: string|null, snapshot_id?: int|string|null} $validated */
        $validated = $request->validate([
            'severity' => ['nullable', 'string', 'in:error,warning,info'],
            'snapshot_id' => ['nullable', 'integer', 'min:1'],
        ]);
        $snapshotId = $this->nullablePositiveInteger($validated['snapshot_id'] ?? null);

        $query = DB::table('game_catalog_validation_findings as findings')
            ->leftJoin('game_catalog_snapshots as snapshots', 'snapshots.id', '=', 'findings.snapshot_id')
            ->orderByDesc('findings.id');

        if (isset($validated['severity'])) {
            $query->where('findings.severity', $validated['severity']);
        }
        if ($snapshotId !== null) {
            $query->where('findings.snapshot_id', $snapshotId);
        }

        return view('game-catalog.admin.findings', [
            'selectedSeverity' => $validated['severity'] ?? '',
            'selectedSnapshotId' => $snapshotId,
            'findings' => $query->limit(200)->get([
                'findings.id',
                'findings.snapshot_id',
                'findings.import_run_id',
                'findings.severity',
                'findings.code',
                'findings.path',
                'findings.message',
                'findings.created_at',
                'snapshots.content_sha256',
            ]),
            'snapshots' => DB::table('game_catalog_snapshots')->orderByDesc('id')->limit(100)->get(['id', 'content_sha256']),
        ]);
    }

    public function diff(Request $request, CatalogSnapshotDiffService $diffService): View
    {
        /** @var array{snapshot_a?: int|string|null, snapshot_b?: int|string|null} $validated */
        $validated = $request->validate([
            'snapshot_a' => ['nullable', 'integer', 'min:1', 'required_with:snapshot_b'],
            'snapshot_b' => ['nullable', 'integer', 'min:1', 'required_with:snapshot_a', 'different:snapshot_a'],
        ]);
        $snapshotA = $this->nullablePositiveInteger($validated['snapshot_a'] ?? null);
        $snapshotB = $this->nullablePositiveInteger($validated['snapshot_b'] ?? null);

        $diff = null;
        $diffError = null;
        if ($snapshotA !== null && $snapshotB !== null) {
            try {
                $diff = $diffService->diff($snapshotA, $snapshotB);
            } catch (RuntimeException $exception) {
                $diffError = $exception->getMessage();
            }
        }

        return view('game-catalog.admin.diff', [
            'selectedSnapshotA' => $snapshotA,
            'selectedSnapshotB' => $snapshotB,
            'snapshots' => DB::table('game_catalog_snapshots')->orderByDesc('id')->limit(100)->get(['id', 'content_sha256', 'status']),
            'diff' => $diff instanceof CatalogSnapshotDiff ? $diff : null,
            'diffError' => $diffError,
        ]);
    }

    private function snapshotQuery(): Builder
    {
        return DB::table('game_catalog_snapshots as snapshots')
            ->join('game_catalog_releases as runtime_release', 'runtime_release.id', '=', 'snapshots.runtime_release_id')
            ->join('game_catalog_releases as content_target', 'content_target.id', '=', 'snapshots.content_target_release_id')
            ->leftJoin('game_catalog_releases as verified_release', 'verified_release.id', '=', 'snapshots.verified_content_through_release_id')
            ->leftJoin('game_catalog_releases as contains_release', 'contains_release.id', '=', 'snapshots.contains_content_through_release_id')
            ->orderByDesc('snapshots.id')
            ->select([
                'snapshots.id',
                'snapshots.contract_version',
                'snapshots.schema_version',
                'snapshots.content_sha256',
                'snapshots.canary_commit_sha',
                'snapshots.datapack_commit_sha',
                'snapshots.protocol_profile',
                'snapshots.status',
                'snapshots.entity_count',
                'snapshots.relation_count',
                'snapshots.generated_at',
                'snapshots.imported_at',
                'runtime_release.display_label as runtime_release',
                'content_target.display_label as content_target_release',
                'verified_release.display_label as verified_content_through_release',
                'contains_release.display_label as contains_content_through_release',
            ]);
    }

    private function profileQuery(): Builder
    {
        return DB::table('game_catalog_profiles as profiles')
            ->join('game_catalog_releases as target_release', 'target_release.id', '=', 'profiles.target_release_id')
            ->leftJoin('game_catalog_snapshots as active_snapshot', 'active_snapshot.id', '=', 'profiles.active_snapshot_id')
            ->orderBy('profiles.key')
            ->select([
                'profiles.id',
                'profiles.key',
                'profiles.name',
                'profiles.active_snapshot_id',
                'profiles.protocol_profile',
                'profiles.complete_only',
                'profiles.completeness_policy_key',
                'profiles.availability_policy_key',
                'profiles.validation_policy_key',
                'profiles.public_enabled',
                'profiles.allow_backports',
                'profiles.lock_version',
                'target_release.display_label as target_release',
                'active_snapshot.content_sha256 as active_snapshot_sha256',
                'active_snapshot.status as active_snapshot_status',
            ]);
    }

    private function nullablePositiveInteger(mixed $value): ?int
    {
        if ($value === null || $value === '') {
            return null;
        }
        if (is_int($value) && $value > 0) {
            return $value;
        }
        if (is_string($value) && preg_match('/^[1-9][0-9]*$/D', $value) === 1) {
            return (int) $value;
        }

        throw new RuntimeException('Expected a positive integer query value after validation.');
    }
}
