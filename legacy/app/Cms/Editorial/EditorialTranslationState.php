<?php

namespace App\Cms\Editorial;

enum EditorialTranslationState: string
{
    case Missing = 'missing';
    case Incomplete = 'incomplete';
    case Draft = 'draft';
    case Published = 'published';
    case Stale = 'stale';
}
