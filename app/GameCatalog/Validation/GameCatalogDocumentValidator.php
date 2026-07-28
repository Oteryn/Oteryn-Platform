<?php

namespace App\GameCatalog\Validation;

use App\GameCatalog\Contract\GameCatalogContract;
use JsonException;
use RuntimeException;

final class GameCatalogDocumentValidator
{
    public function __construct(
        private readonly ?string $schemaPath = null,
        private readonly ?JsonSchemaSubsetValidator $schemaValidator = null,
        private readonly ?GameCatalogSemanticValidator $semanticValidator = null,
    ) {}

    public function validatePath(string $path, ?string $expectedSha256 = null): ValidatedCatalogDocument
    {
        if ($path === '' || ! is_file($path) || is_link($path)) {
            throw new CatalogValidationException([$this->finding('file.invalid', '$', 'Snapshot path must reference a regular file.')]);
        }

        $size = filesize($path);
        if (! is_int($size) || $size < 1 || $size > GameCatalogContract::MAX_DOCUMENT_BYTES) {
            throw new CatalogValidationException([$this->finding('file.size', '$', 'Snapshot file size is outside the configured bounds.')]);
        }

        $raw = file_get_contents($path);
        if (! is_string($raw) || strlen($raw) !== $size) {
            throw new CatalogValidationException([$this->finding('file.read', '$', 'Snapshot file could not be read completely.')]);
        }

        $contentSha256 = hash('sha256', $raw);
        $this->verifyExpectedHash($contentSha256, $expectedSha256, '$/sha256');
        $this->verifySidecar($path, $contentSha256);

        $schemaRaw = $this->readVerifiedSchema();

        try {
            $schema = json_decode($schemaRaw, true, 512, JSON_THROW_ON_ERROR);
            $schemaDocument = json_decode($raw, false, 512, JSON_THROW_ON_ERROR);
            $document = json_decode($raw, true, 512, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new CatalogValidationException([$this->finding('json.invalid', '$', 'Snapshot or shared schema contains invalid UTF-8 or JSON: '.$exception->getMessage())]);
        }

        if (! is_array($schema) || ! is_object($schemaDocument) || ! is_array($document)) {
            throw new CatalogValidationException([$this->finding('json.root', '$', 'Snapshot and schema roots must be JSON objects.')]);
        }

        $schemaFindings = ($this->schemaValidator ?? new JsonSchemaSubsetValidator(GameCatalogContract::MAX_VALIDATION_FINDINGS))
            ->validate($schemaDocument, $schema);
        if ($schemaFindings !== []) {
            throw new CatalogValidationException(array_slice($schemaFindings, 0, GameCatalogContract::MAX_VALIDATION_FINDINGS));
        }

        $semanticFindings = ($this->semanticValidator ?? new GameCatalogSemanticValidator)->validate($document);
        if ($semanticFindings !== []) {
            throw new CatalogValidationException(array_slice($semanticFindings, 0, GameCatalogContract::MAX_VALIDATION_FINDINGS));
        }

        return new ValidatedCatalogDocument($document, $contentSha256, $size);
    }

    private function readVerifiedSchema(): string
    {
        $path = $this->schemaPath ?? base_path(GameCatalogContract::SCHEMA_PATH);
        if (! is_file($path) || is_link($path)) {
            throw new RuntimeException('The pinned Game Catalog schema file is unavailable.');
        }

        $raw = file_get_contents($path);
        if (! is_string($raw)) {
            throw new RuntimeException('The pinned Game Catalog schema file could not be read.');
        }

        $actual = hash('sha256', $raw);
        if (! hash_equals(GameCatalogContract::SCHEMA_SHA256, $actual)) {
            throw new RuntimeException('The pinned Game Catalog schema hash does not match the cross-repository contract.');
        }

        return $raw;
    }

    private function verifySidecar(string $path, string $actual): void
    {
        $sidecar = $path.'.sha256';
        if (! file_exists($sidecar)) {
            return;
        }
        if (! is_file($sidecar) || is_link($sidecar)) {
            throw new CatalogValidationException([$this->finding('hash.sidecar', '$/sha256', 'SHA-256 sidecar is not a regular file.')]);
        }

        $value = file_get_contents($sidecar);
        if (! is_string($value) || preg_match('/^([0-9a-f]{64})(?:\s+[^\r\n]+)?\s*$/D', $value, $matches) !== 1) {
            throw new CatalogValidationException([$this->finding('hash.sidecar', '$/sha256', 'SHA-256 sidecar has an invalid format.')]);
        }

        $this->verifyExpectedHash($actual, $matches[1], '$/sha256');
    }

    private function verifyExpectedHash(string $actual, ?string $expected, string $path): void
    {
        if ($expected === null) {
            return;
        }
        if (preg_match('/^[0-9a-f]{64}$/D', $expected) !== 1 || ! hash_equals($expected, $actual)) {
            throw new CatalogValidationException([$this->finding('hash.mismatch', $path, 'Snapshot SHA-256 does not match the expected value.')]);
        }
    }

    /** @return array{code: string, path: string, message: string} */
    private function finding(string $code, string $path, string $message): array
    {
        return compact('code', 'path', 'message');
    }
}
