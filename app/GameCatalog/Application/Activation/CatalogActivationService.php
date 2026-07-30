<?php

namespace App\GameCatalog\Application\Activation;

use App\GameCatalog\Application\Configuration\CatalogConfiguration;
use App\GameCatalog\Infrastructure\Persistence\CatalogDatabaseRow;
use Carbon\CarbonImmutable;
use Illuminate\Database\ConnectionInterface;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;

final class CatalogActivationService
{
    public function __construct(
        private readonly VisibilityProjector $visibilityProjector,
        private readonly ConnectionInterface $database,
    ) {}

    public function activate(int $snapshotId, string $profileKey, bool $rollback = false, ?int $actorIdentityId = null): CatalogActivationResult
    {
        return $this->database->transaction(function () use ($snapshotId, $profileKey, $rollback, $actorIdentityId): CatalogActivationResult {
            $profile = DB::table('game_catalog_profiles as profiles')
                ->join('game_catalog_releases as target_release', 'target_release.id', '=', 'profiles.target_release_id')
                ->where('profiles.key', $profileKey)->lockForUpdate()->first([
                    'profiles.id', 'profiles.key', 'profiles.active_snapshot_id', 'profiles.protocol_profile', 'profiles.complete_only',
                    'profiles.allow_backports', 'profiles.lock_version', 'target_release.release_order as target_release_order',
                ]);
            if ($profile === null) {
                throw new RuntimeException("Game Catalog profile '{$profileKey}' does not exist.");
            }
            $profileRow = CatalogDatabaseRow::from($profile);

            $snapshot = DB::table('game_catalog_snapshots as snapshots')
                ->join('game_catalog_releases as content_target', 'content_target.id', '=', 'snapshots.content_target_release_id')
                ->leftJoin('game_catalog_releases as verified_content', 'verified_content.id', '=', 'snapshots.verified_content_through_release_id')
                ->leftJoin('game_catalog_releases as contains_content', 'contains_content.id', '=', 'snapshots.contains_content_through_release_id')
                ->where('snapshots.id', $snapshotId)->first([
                    'snapshots.id', 'snapshots.status', 'snapshots.contract_version', 'snapshots.schema_version', 'snapshots.protocol_profile',
                    'snapshots.entity_count', 'snapshots.relation_count', 'content_target.release_order as content_target_order',
                    'verified_content.release_order as verified_content_order', 'contains_content.release_order as contains_content_order',
                ]);
            if ($snapshot === null) {
                throw new RuntimeException("Game Catalog snapshot '{$snapshotId}' does not exist.");
            }
            $snapshotRow = CatalogDatabaseRow::from($snapshot);
            if ($snapshotRow->string('status') !== 'validated') {
                throw new RuntimeException('Only validated immutable Game Catalog snapshots can be activated.');
            }
            $schemaContract = CatalogConfiguration::schemaContract($snapshotRow->string('schema_version'));
            if ($snapshotRow->string('contract_version') !== CatalogConfiguration::string('game-catalog.contract') || $schemaContract === null) {
                throw new RuntimeException('The Game Catalog snapshot contract is incompatible with this Platform build.');
            }
            if (! $schemaContract['activatable']) {
                throw new RuntimeException('This Game Catalog schema is supported for inactive import and review only; activation is not authorized.');
            }
            $profileProtocol = $profileRow->nullableString('protocol_profile');
            if ($profileProtocol !== null && $profileProtocol !== $snapshotRow->string('protocol_profile')) {
                throw new RuntimeException('The Game Catalog snapshot protocol profile is incompatible with the target profile.');
            }
            $contentBoundary = $snapshotRow->nullableInt('contains_content_order') ?? $snapshotRow->int('content_target_order');
            if ($profileRow->int('target_release_order') > $contentBoundary) {
                throw new RuntimeException('The Game Catalog snapshot does not declare content through the profile target release.');
            }
            $verifiedBoundary = $snapshotRow->nullableInt('verified_content_order');
            if ($verifiedBoundary === null) {
                throw new RuntimeException('The Game Catalog snapshot has no verified-content boundary and cannot be activated.');
            }
            if ($profileRow->int('target_release_order') > $verifiedBoundary) {
                throw new RuntimeException('The Game Catalog snapshot is not verified through the profile target release.');
            }
            $persistedEntityCount = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
            $persistedRelationCount = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();
            if ($persistedEntityCount !== $snapshotRow->int('entity_count') || $persistedRelationCount !== $snapshotRow->int('relation_count')) {
                throw new RuntimeException('The Game Catalog snapshot persistence verification failed before activation.');
            }

            $computedAt = CarbonImmutable::now('UTC');
            $projection = $this->visibilityProjector->rebuild(
                profileId: $profileRow->int('id'), snapshotId: $snapshotId, targetReleaseOrder: $profileRow->int('target_release_order'),
                completeOnly: $profileRow->bool('complete_only'), allowBackports: $profileRow->bool('allow_backports'), computedAt: $computedAt,
            );
            DB::table('game_catalog_profiles')->where('id', $profileRow->int('id'))->update([
                'active_snapshot_id' => $snapshotId, 'lock_version' => $profileRow->int('lock_version') + 1, 'updated_at' => $computedAt,
            ]);
            DB::table('game_catalog_audit_events')->insert([
                'action' => $rollback ? 'snapshot.rollback' : 'snapshot.activate', 'actor_identity_id' => $actorIdentityId,
                'profile_id' => $profileRow->int('id'), 'snapshot_id' => $snapshotId,
                'metadata' => $this->json([
                    'previous_snapshot_id' => $profileRow->nullableInt('active_snapshot_id'), 'target_snapshot_id' => $snapshotId,
                    'visible_entity_count' => $projection->visibleEntityCount, 'visible_relation_count' => $projection->visibleRelationCount, 'rollback' => $rollback,
                ]),
                'created_at' => $computedAt,
            ]);
            return new CatalogActivationResult(
                profileId: $profileRow->int('id'), profileKey: $profileRow->string('key'), snapshotId: $snapshotId,
                previousSnapshotId: $profileRow->nullableInt('active_snapshot_id'), visibleEntityCount: $projection->visibleEntityCount,
                visibleRelationCount: $projection->visibleRelationCount, rollback: $rollback,
            );
        }, 3);
    }

    private function json(mixed $value): string
    {
        try {
            return json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (JsonException $exception) {
            throw new RuntimeException('Game Catalog activation audit metadata could not be encoded.', previous: $exception);
        }
    }
}
