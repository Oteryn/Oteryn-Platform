<?php

namespace App\Http\Controllers\PublicPortal;

use App\Http\Controllers\Controller;
use App\PublicPortal\HomePageQuery;
use App\PublicPortal\HomepageTemplates\HomepageTemplateRegistry;
use App\PublicPortal\HomepageTemplates\HomepageTemplateStore;
use Illuminate\View\View;

final readonly class PublicHomeController extends Controller
{
    public function __construct(
        private HomePageQuery $query,
        private HomepageTemplateRegistry $templates,
        private HomepageTemplateStore $templateStore,
    ) {}

    public function __invoke(): View
    {
        $snapshot = $this->templateStore->snapshot();

        return view($this->templates->view($snapshot->activeKey), [
            'homePage' => $this->query->get(),
        ]);
    }
}
