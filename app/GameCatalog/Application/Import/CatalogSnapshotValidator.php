<?php

namespace App\GameCatalog\Application\Import;

use App\GameCatalog\Application\Configuration\CatalogConfiguration;
use App\GameCatalog\Domain\CatalogValidationFinding;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use App\GameCatalog\Infrastructure\Json\BundledJsonSchemaValidator;
use App\GameCatalog\Infrastructure\Json\DuplicateJsonKeyDetector;
use JsonException;
use RuntimeException;
use Throwable;

/** @phpstan-import-type CatalogPayload from ValidatedCatalogSnapshot */
final class CatalogSnapshotValidator
{
    public function __construct(
        private readonly DuplicateJsonKeyDetector $duplicateJsonKeyDetector,
        private readonly BundledJsonSchemaValidator $schemaValidator,
        private readonly CatalogSemanticValidator $semanticValidator,
    ) {}

    public function validate(string $path, ?string $expectedSha256 = null): ValidatedCatalogSnapshot
    {
        $maximumFileBytes = CatalogConfiguration::positiveInt('game-catalog.limits.file_bytes', 268_435_456);

        $this->assertRegularReadableFile($path, 'The Game Catalog snapshot path must be a readable regular file.');
        $fileSize = filesize($path);
        if (! is_int($fileSize)) {
            throw $this->failure('input.file_size_unavailable', 'The Game Catalog snapshot size could not be determined.', '$file');
        }
        if ($fileSize > $maximumFileBytes) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding(
                    severity: 'error',
                    code: 'input.file_too_large',
                    message: 'The Game Catalog snapshot exceeds the configured file-size limit.',
                    path: '$file',
                    context: ['file_size' => $fileSize, 'maximum' => $maximumFileBytes],
                )],
                fileSize: $fileSize,
            );
        }

        $contents = file_get_contents($path);
        if (! is_string($contents)) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'input.read_failed', 'The Game Catalog snapshot could not be read.', '$file')],
                fileSize: $fileSize,
            );
        }

        $contentSha256 = hash('sha256', $contents);
        if ($expectedSha256 !== null) {
            $normalizedExpected = strtolower($expectedSha256);
            if (preg_match('/^[0-9a-f]{64}$/D', $normalizedExpected) !== 1 || ! hash_equals($normalizedExpected, $contentSha256)) {
                throw new CatalogValidationException(
                    findings: [new CatalogValidationFinding(
                        severity: 'error',
                        code: 'input.hash_mismatch',
                        message: 'The Game Catalog snapshot SHA-256 does not match the expected value.',
                        path: '$file',
                        context: ['actual_sha256' => $contentSha256],
                    )],
                    contentSha256: $contentSha256,
                    fileSize: $fileSize,
                );
            }
        }

        if (str_starts_with($contents, "\xEF\xBB\xBF")) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'input.utf8_bom', 'UTF-8 BOM is not allowed.', '$file')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }

        if (! json_validate($contents, 128)) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'input.invalid_json', json_last_error_msg(), '$')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }

        try {
            $duplicates = $this->duplicateJsonKeyDetector->find($contents);
        } catch (Throwable) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'input.json_scan_failed', 'The JSON object-key scan failed.', '$')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }
        if ($duplicates !== []) {
            $maximumFindings = CatalogConfiguration::positiveInt('game-catalog.limits.validation_findings', 2_000);
            $findings = [];
            foreach (array_slice($duplicates, 0, $maximumFindings) as $duplicate) {
                $findings[] = new CatalogValidationFinding('error', 'input.duplicate_json_key', 'Duplicate JSON object key.', $duplicate);
            }
            throw new CatalogValidationException($findings, $contentSha256, $fileSize);
        }

        try {
            $objectDocument = json_decode($contents, false, 128, JSON_THROW_ON_ERROR);
            $payload = json_decode($contents, true, 128, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'input.invalid_json', $exception->getMessage(), '$')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }

        if (! is_array($payload) || array_is_list($payload)) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'schema.root', 'The Game Catalog snapshot root must be an object.', '$')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }

        $schemaVersion = $payload['schema_version'] ?? null;
        if (! is_string($schemaVersion) || $schemaVersion === '') {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'contract.schema_version_missing', 'The Game Catalog schema version is missing.', '$.schema_version')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }
        $schemaContract = CatalogConfiguration::schemaContract($schemaVersion);
        if ($schemaContract === null) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'contract.schema_version_unsupported', 'The Game Catalog schema version is not supported by this Platform build.', '$.schema_version')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }
        $schemaPath = $schemaContract['path'];
        $expectedSchemaSha256 = $schemaContract['sha256'];
        $this->assertRegularReadableFile($schemaPath, 'The bundled Game Catalog schema is unavailable.');
        $schemaSha256 = hash_file('sha256', $schemaPath);
        if (! is_string($schemaSha256) || ! hash_equals($expectedSchemaSha256, $schemaSha256)) {
            throw new CatalogValidationException(
                findings: [new CatalogValidationFinding('error', 'contract.schema_hash_mismatch', 'The bundled Game Catalog schema does not match the registered contract hash.', '$schema')],
                contentSha256: $contentSha256,
                fileSize: $fileSize,
            );
        }

        $schemaContents = file_get_contents($schemaPath);
        if (! is_string($schemaContents)) {
            throw new RuntimeException('The bundled Game Catalog schema could not be read after its hash was verified.');
        }

        $schemaFindings = $this->schemaValidator->validate($objectDocument, $schemaContents);
        if ($schemaFindings !== []) {
            throw new CatalogValidationException($schemaFindings, $contentSha256, $fileSize);
        }

        $this->assertSchemaValidatedPayload($payload);

        $semanticFindings = $this->semanticValidator->validate($payload);
        if ($semanticFindings !== []) {
            throw new CatalogValidationException($semanticFindings, $contentSha256, $fileSize);
        }

        return new ValidatedCatalogSnapshot(
            payload: $payload,
            contentSha256: $contentSha256,
            schemaSha256: $schemaSha256,
            fileSize: $fileSize,
            sourceLabel: basename($path),
        );
    }

    /**
     * The bundled fixed schema has already validated every nested field before
     * this assertion is reached. These top-level checks keep the static type
     * boundary explicit and fail closed if the schema validator contract ever
     * changes unexpectedly.
     *
     * @param  array<mixed, mixed>  $payload
     *
     * @phpstan-assert CatalogPayload $payload
     */
    private function assertSchemaValidatedPayload(array $payload): void
    {
        $snapshot = $payload['snapshot'] ?? null;
        $releases = $payload['releases'] ?? null;
        $entities = $payload['entities'] ?? null;
        $relations = $payload['relations'] ?? null;

        if (
            ! is_string($payload['contract'] ?? null)
            || ! is_string($payload['schema_version'] ?? null)
            || ! is_array($snapshot)
            || array_is_list($snapshot)
            || ! is_array($releases)
            || ! array_is_list($releases)
            || ! is_array($entities)
            || ! array_is_list($entities)
            || ! is_array($relations)
            || ! array_is_list($relations)
        ) {
            throw new RuntimeException('The schema-validated Game Catalog payload has an unexpected PHP shape.');
        }
    }

    private function assertRegularReadableFile(string $path, string $message): void
    {
        if ($path === '' || ! is_file($path) || ! is_readable($path)) {
            throw $this->failure('input.file_unavailable', $message, '$file');
        }
    }

    private function failure(string $code, string $message, string $path): CatalogValidationException
    {
        return new CatalogValidationException([
            new CatalogValidationFinding(
                severity: 'error',
                code: $code,
                message: $message,
                path: $path,
            ),
        ]);
    }
}
