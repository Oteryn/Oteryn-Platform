<?php

namespace App\PublicPortal\ViewModels;

use App\Cms\Models\NewsPost;
use App\PublicPortal\PublicContentState;

final readonly class HomeNewsSummary
{
    /**
     * @param  list<NewsPost>  $posts
     */
    public function __construct(
        public PublicContentState $state,
        public array $posts,
    ) {}
}
