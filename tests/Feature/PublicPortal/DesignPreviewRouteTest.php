<?php

namespace Tests\Feature\PublicPortal;

use Tests\TestCase;

final class DesignPreviewRouteTest extends TestCase
{
    public function test_obsolete_public_homepage_design_preview_is_not_routable(): void
    {
        $this->get('/design/home-v2')->assertNotFound();
    }
}
