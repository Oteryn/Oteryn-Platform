<?php

namespace App\GameCatalog\Validation;

final readonly class ValidatedCatalogDocument
{
    /**
     * @param  array<string, mixed>  $document
     */
    public function __construct(
        public array $document,
        public string $contentSha256,
        public int $byteSize,
    ) {}
}
