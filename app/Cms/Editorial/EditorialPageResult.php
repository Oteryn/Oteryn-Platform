<?php

namespace App\Cms\Editorial;

use App\Cms\Models\ManagedPage;

final readonly class EditorialPageResult
{
    public function __construct(
        public EditorialPageKey $key,
        public EditorialPageState $state,
        public ?ManagedPage $page,
    ) {}
}
