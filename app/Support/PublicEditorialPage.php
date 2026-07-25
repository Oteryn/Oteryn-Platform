<?php

namespace App\Support;

use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Editorial\EditorialPageQuery;
use App\Cms\Editorial\EditorialPageState;
use Illuminate\Http\Response;

final class PublicEditorialPage
{
    public function __construct(
        private readonly EditorialPageQuery $pages,
        private readonly ApprovedSupportLinks $links,
    ) {}

    public function render(EditorialPageKey $key, bool $includeSupportLinks): Response
    {
        $result = $this->pages->find($key);

        return response()->view('support.editorial.show', [
            'key' => $key,
            'result' => $result,
            'supportLinks' => $includeSupportLinks ? $this->links->all() : [],
        ], $result->state === EditorialPageState::Published ? 200 : 404);
    }
}
