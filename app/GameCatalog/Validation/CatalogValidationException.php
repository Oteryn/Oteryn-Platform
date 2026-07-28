<?php

namespace App\GameCatalog\Validation;

use RuntimeException;

final class CatalogValidationException extends RuntimeException
{
    /**
     * @param  list<array{code: string, path: string, message: string}>  $findings
     */
    public function __construct(private readonly array $findings)
    {
        parent::__construct($findings[0]['message'] ?? 'Game Catalog validation failed.');
    }

    /**
     * @return list<array{code: string, path: string, message: string}>
     */
    public function findings(): array
    {
        return $this->findings;
    }
}
