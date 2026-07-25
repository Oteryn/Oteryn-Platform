<?php

namespace App\Downloads\ViewModels;

use App\Downloads\DownloadCenterState;
use App\Downloads\Models\ClientRelease;

final readonly class DownloadCenterViewModel
{
    /**
     * @var list<ClientRelease>
     */
    public array $releases;

    /**
     * @param  list<ClientRelease>  $releases
     */
    public function __construct(
        public DownloadCenterState $state,
        array $releases,
        public ?string $platform,
    ) {
        $this->releases = $releases;
    }
}
