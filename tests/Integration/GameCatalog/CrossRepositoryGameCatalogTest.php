<?php

namespace Tests\Integration\GameCatalog;

use App\GameCatalog\Application\Activation\CatalogActivationService;
use App\GameCatalog\Application\Import\CatalogImportService;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;
use Tests\TestCase;

final class CrossRepositoryGameCatalogTest extends TestCase
{
    use RefreshDatabase;

    /**
     * @throws JsonException
     */
    public function test_generated_canary_snapshots_import_activate_and_roll_back_in_staging(): void
    {
        $baselinePath = $this->requiredFile('GAME_CATALOG_BASELINE_PATH');
        $candidatePath = $this->requiredFile('GAME_CATALOG_CANDIDATE_PATH');
        $expectedCanarySha = $this->requiredEnvironmentValue('GAME_CATALOG_CANARY_SHA');

        $baselineDocument = $this->decodeDocument($baselinePath);
        $candidateDocument = $this->decodeDocument($candidatePath);
        $baselineSnapshot = $this->snapshotMetadata($baselineDocument);
        $candidateSnapshot = $this->snapshotMetadata($candidateDocument);

        $this->assertSame('oteryn.game-catalog', $baselineDocument['contract'] ?? null);
        $this->assertSame('1.0.0', $baselineDocument['schema_version'] ?? null);
        $this->assertSame('oteryn.game-catalog', $candidateDocument['contract'] ?? null);
        $this->assertSame('1.0.0', $candidateDocument['schema_version'] ?? null);
        $this->assertSame($expectedCanarySha, $baselineSnapshot['canary_commit_sha'] ?? null);
        $this->assertSame($expectedCanarySha, $candidateSnapshot['canary_commit_sha'] ?? null);
        $this->assertNotSame($baselineSnapshot['generated_at'] ?? null, $candidateSnapshot['generated_at'] ?? null);

        $targetRelease = $candidateSnapshot['content_target_release'] ?? null;
        $protocolProfile = $candidateSnapshot['protocol_profile'] ?? null;
        if (! is_string($targetRelease) || $targetRelease === '') {
            throw new RuntimeException('The generated Canary candidate has no content target release.');
        }
        if (! is_string($protocolProfile) || $protocolProfile === '') {
            throw new RuntimeException('The generated Canary candidate has no protocol profile.');
        }
        $this->assertSame($targetRelease, $baselineSnapshot['content_target_release'] ?? null);
        $this->assertSame($protocolProfile, $baselineSnapshot['protocol_profile'] ?? null);

        $importer = app(CatalogImportService::class);
        $baseline = $importer->import($baselinePath, $this->sha256($baselinePath));
        $candidate = $importer->import($candidatePath, $this->sha256($candidatePath));

        $this->assertFalse($baseline->deduplicated);
        $this->assertFalse($candidate->deduplicated);
        $this->assertNotSame($baseline->contentSha256, $candidate->contentSha256);

        $releaseId = $this->integerDatabaseValue(
            DB::table('game_catalog_releases')->where('key', $targetRelease)->value('id'),
            'target release',
        );
        $now = CarbonImmutable::now('UTC');
        $profileId = (int) DB::table('game_catalog_profiles')->insertGetId([
            'key' => 'staging-canary-cross-repo',
            'name' => 'Canary cross-repository staging proof',
            'target_release_id' => $releaseId,
            'active_snapshot_id' => null,
            'protocol_profile' => $protocolProfile,
            'complete_only' => true,
            'completeness_policy_key' => 'complete-only',
            'availability_policy_key' => 'public-proven',
            'validation_policy_key' => 'validated-snapshot',
            'public_enabled' => false,
            'allow_backports' => false,
            'lock_version' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        $publicProfileBefore = DB::table('game_catalog_profiles')->where('key', 'public')->first();
        $activation = app(CatalogActivationService::class);

        $baselineActivation = $activation->activate($baseline->snapshotId, 'staging-canary-cross-repo');
        $this->assertNull($baselineActivation->previousSnapshotId);
        $this->assertSame($baseline->snapshotId, $baselineActivation->snapshotId);
        $this->assertFalse($baselineActivation->rollback);

        $candidateActivation = $activation->activate($candidate->snapshotId, 'staging-canary-cross-repo');
        $this->assertSame($baseline->snapshotId, $candidateActivation->previousSnapshotId);
        $this->assertSame($candidate->snapshotId, $candidateActivation->snapshotId);
        $this->assertFalse($candidateActivation->rollback);

        $rollback = $activation->activate($baseline->snapshotId, 'staging-canary-cross-repo', rollback: true);
        $this->assertSame($candidate->snapshotId, $rollback->previousSnapshotId);
        $this->assertSame($baseline->snapshotId, $rollback->snapshotId);
        $this->assertTrue($rollback->rollback);

        $profile = DB::table('game_catalog_profiles')->where('id', $profileId)->first();
        $this->assertNotNull($profile);
        $this->assertSame($baseline->snapshotId, (int) $profile->active_snapshot_id);
        $this->assertSame(3, (int) $profile->lock_version);
        $this->assertSame(0, (int) $profile->public_enabled);

        $this->assertSame(
            ['snapshot.activate', 'snapshot.activate', 'snapshot.rollback'],
            DB::table('game_catalog_audit_events')
                ->where('profile_id', $profileId)
                ->orderBy('id')
                ->pluck('action')
                ->all(),
        );
        $this->assertEquals(
            $publicProfileBefore,
            DB::table('game_catalog_profiles')->where('key', 'public')->first(),
        );

        $this->assertGreaterThan(0, DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $candidate->snapshotId)->count());
        $this->assertGreaterThan(0, DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $candidate->snapshotId)->count());
    }

    private function requiredFile(string $name): string
    {
        $value = $this->requiredEnvironmentValue($name);
        if (! is_file($value)) {
            throw new RuntimeException("{$name} does not reference a readable file.");
        }

        return $value;
    }

    private function requiredEnvironmentValue(string $name): string
    {
        $value = getenv($name);
        if (! is_string($value) || trim($value) === '') {
            throw new RuntimeException("{$name} is required for the cross-repository staging proof.");
        }

        return $value;
    }

    /** @return array<string, mixed> */
    private function decodeDocument(string $path): array
    {
        $contents = file_get_contents($path);
        if (! is_string($contents)) {
            throw new RuntimeException("Unable to read {$path}.");
        }

        $decoded = json_decode($contents, true, flags: JSON_THROW_ON_ERROR);
        if (! is_array($decoded)) {
            throw new RuntimeException("{$path} is not a JSON object.");
        }

        return $decoded;
    }

    /**
     * @param  array<string, mixed>  $document
     * @return array<string, mixed>
     */
    private function snapshotMetadata(array $document): array
    {
        $snapshot = $document['snapshot'] ?? null;
        if (! is_array($snapshot)) {
            throw new RuntimeException('The generated Canary document has no snapshot metadata object.');
        }

        return $snapshot;
    }

    private function sha256(string $path): string
    {
        $sha256 = hash_file('sha256', $path);
        if (! is_string($sha256)) {
            throw new RuntimeException("Unable to hash {$path}.");
        }

        return $sha256;
    }

    private function integerDatabaseValue(mixed $value, string $description): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new RuntimeException("Expected an integer-compatible {$description} id.");
    }
}
