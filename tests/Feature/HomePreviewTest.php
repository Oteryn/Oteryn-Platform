<?php

namespace Tests\Feature;

use Tests\TestCase;

final class HomePreviewTest extends TestCase
{
    public function test_obsolete_public_homepage_design_preview_is_not_available(): void
    {
        $this->get('/design/home-v2')->assertNotFound();
    }

    public function test_production_homepage_uses_the_approved_visual_foundation_without_preview_notice(): void
    {
        $response = $this->get('/');

        $response->assertOk();
        $response->assertSee('Answer the call of Oteryn');
        $response->assertSee('css/home-preview.css', false);
        $response->assertSee('css/home-production.css', false);
        $response->assertDontSee('Design preview.');
        $response->assertDontSee('noindex,nofollow', false);
    }
}
