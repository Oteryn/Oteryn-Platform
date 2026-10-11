<?php

namespace App\GameCatalog\Application\Import\Native;

use App\GameCatalog\Application\Configuration\CatalogConfiguration;
use App\GameCatalog\Domain\CatalogValidationFinding;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use App\GameCatalog\Infrastructure\Persistence\CatalogDatabaseRow;
use Carbon\CarbonImmutable;
use Illuminate\Database\ConnectionInterface;
use JsonException;

/**
 * Accepts a locked native catalogue export behind `game-catalog.native_import.enabled` (default off).
 *
 * Acceptance records import-run evidence only: no snapshot, entity, relation or profile rows are written
 * and nothing is activated, because the native snapshot persistence shape is still undecided (#1460).
 * Storing native provenance in the legacy snapshot row would need invented legacy release values.
 */
final class NativeCatalogImportService
{
    public const string ENABLED_KEY = 'game-catalog.native_import.enabled';

    public const string ACCEPTED_STATUS = 'native_accepted';

    public function __construct(
        private readonly NativeCatalogEnvelopeValidator $validator,
        private readonly ConnectionInterface $database,
    ) {}

    public function import(string $path, ?string $expectedSha256 = null): NativeCatalogImportResult
    {
        if (! CatalogConfiguration::boolean(self::ENABLED_KEY, false)) {
            throw new CatalogValidationException([
                new CatalogValidationFinding(
                    severity: 'error',
                    code: 'native.import.disabled',
                    message: 'Native Game Catalog import is disabled by configuration.',
                    path: '$',
                ),
            ]);
        }

        $validated = $this->validator->validate($path, $expectedSha256);
        $payload = $validated->payload;

        return $this->database->transaction(function () use ($validated, $payload, $path): NativeCatalogImportResult {
            $existing = $this->database->table('game_catalog_import_runs')
                ->where('content_sha256', $validated->artifactSha256)
                ->where('status', self::ACCEPTED_STATUS)
                ->orderBy('id')
                ->lockForUpdate()
                ->first(['id']);

            if ($existing !== null) {
                return new NativeCatalogImportResult(
                    importRunId: CatalogDatabaseRow::from($existing)->int('id'),
                    artifactSha256: $validated->artifactSha256,
                    nativeSnapshotId: $payload['snapshot_id'],
                    sourceRevision: $payload['source_revision'],
                    deduplicated: true,
                );
            }

            $now = CarbonImmutable::now('UTC');
            $runId = (int) $this->database->table('game_catalog_import_runs')->insertGetId([
                'content_sha256' => $validated->artifactSha256,
                'snapshot_id' => null,
                'status' => self::ACCEPTED_STATUS,
                'source_label' => mb_substr(basename($path), 0, 255, 'UTF-8'),
                'file_size' => $validated->fileSize,
                'finding_count' => 0,
                'started_at' => $now,
                'finished_at' => $now,
                'summary' => $this->json([
                    'format' => 'native',
                    'contract_id' => $payload['contract_id'],
                    'schema_version' => $payload['schema_version'],
                    'snapshot_id' => $payload['snapshot_id'],
                    'content_authority_id' => $payload['content_authority_id'],
                    'authority_epoch' => $payload['authority_epoch'],
                    'source_revision' => $payload['source_revision'],
                    'ruleset_id' => $payload['ruleset_id'],
                    'content_profile_id' => $payload['content_profile_id'],
                    'payload_digest' => $payload['payload_digest'],
                    'entity_count' => count($payload['entities']),
                    'relation_count' => count($payload['relations']),
                    'tombstone_count' => count($payload['tombstones']),
                    'snapshot_persisted' => false,
                    'activated' => false,
                ]),
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            return new NativeCatalogImportResult(
                importRunId: $runId,
                artifactSha256: $validated->artifactSha256,
                nativeSnapshotId: $payload['snapshot_id'],
                sourceRevision: $payload['source_revision'],
                deduplicated: false,
            );
        }, 3);
    }

    /** @param array<string, mixed> $value */
    private function json(array $value): string
    {
        try {
            return json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (JsonException $exception) {
            throw new CatalogValidationException([
                new CatalogValidationFinding('error', 'native.import.json_encoding', 'Native import evidence could not be encoded.', '$'),
            ], previous: $exception);
        }
    }
}
