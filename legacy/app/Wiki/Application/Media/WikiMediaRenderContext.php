<?php

namespace App\Wiki\Application\Media;

final readonly class WikiMediaRenderContext
{
    /**
     * @param  array<int, string>  $urlsByMediaId
     */
    public function __construct(private array $urlsByMediaId) {}

    public function urlFor(int $mediaId): ?string
    {
        return $this->urlsByMediaId[$mediaId] ?? null;
    }
}
