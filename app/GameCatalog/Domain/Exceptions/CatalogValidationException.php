<?php

namespace App\GameCatalog\Domain\Exceptions;

use App\GameCatalog\Domain\CatalogValidationFinding;
use RuntimeException;

final class CatalogValidationException extends RuntimeException
{
    /**
     * @param  list<CatalogValidationFinding>  $findings
     */
    public function __construct(
        public readonly array $findings,
        public readonly ?string $contentSha256 = null,
        public readonly ?int $fileSize = null,
    ) {
        parent::__construct($findings[0]->message ?? 'Game Catalog validation failed.');
    }
}
