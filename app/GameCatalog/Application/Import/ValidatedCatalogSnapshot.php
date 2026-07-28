<?php

namespace App\GameCatalog\Application\Import;

final readonly class ValidatedCatalogSnapshot
{
    /**
     * @param  array{
     *   contract: string,
     *   schema_version: string,
     *   snapshot: array<string, mixed>,
     *   releases: list<array<string, mixed>>,
     *   entities: list<array<string, mixed>>,
     *   relations: list<array<string, mixed>>
     * }  $payload
     */
    public function __construct(
        public array $payload,
        public string $contentSha256,
        public int $fileSize,
        public string $sourceLabel,
    ) {}
}
