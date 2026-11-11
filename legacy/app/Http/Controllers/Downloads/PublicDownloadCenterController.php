<?php

namespace App\Http\Controllers\Downloads;

use App\Downloads\DownloadCatalog;
use App\Downloads\PublicDownloadCenterQuery;
use Illuminate\Contracts\View\View;

final readonly class PublicDownloadCenterController
{
    public function __construct(private PublicDownloadCenterQuery $downloads) {}

    public function __invoke(?string $platform = null): View
    {
        return view('downloads.index', [
            'downloadCenter' => $this->downloads->get($platform),
            'platforms' => DownloadCatalog::platforms(),
        ]);
    }
}
