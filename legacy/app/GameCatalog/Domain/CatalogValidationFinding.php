<?php

namespace App\GameCatalog\Domain;

final readonly class CatalogValidationFinding
{
    /**
     * @param  array<string, bool|float|int|string|null>  $context
     */
    public function __construct(
        public string $severity,
        public string $code,
        public string $message,
        public ?string $path = null,
        public array $context = [],
    ) {}

    /**
     * @return array{severity: string, code: string, message: string, path: string|null, context: array<string, bool|float|int|string|null>}
     */
    public function toArray(): array
    {
        return [
            'severity' => $this->severity,
            'code' => $this->code,
            'message' => $this->message,
            'path' => $this->path,
            'context' => $this->context,
        ];
    }
}
