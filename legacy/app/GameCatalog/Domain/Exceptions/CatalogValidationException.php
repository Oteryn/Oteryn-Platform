<?php

namespace App\GameCatalog\Domain\Exceptions;

use App\GameCatalog\Domain\CatalogValidationFinding;
use RuntimeException;
use Throwable;

final class CatalogValidationException extends RuntimeException
{
    /**
     * @param  list<CatalogValidationFinding>  $findings
     */
    public function __construct(
        public readonly array $findings,
        public readonly ?string $contentSha256 = null,
        public readonly ?int $fileSize = null,
        ?Throwable $previous = null,
    ) {
        parent::__construct(
            message: $findings[0]->message ?? 'Game Catalog validation failed.',
            previous: $previous,
        );
    }
}
