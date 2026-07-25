<?php

namespace App\Cms\Editorial;

enum EditorialPageState: string
{
    case Published = 'published';
    case Unpublished = 'unpublished';
    case Missing = 'missing';
}
