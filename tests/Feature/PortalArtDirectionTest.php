<?php

namespace Tests\Feature;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class PortalArtDirectionTest extends TestCase
{
    use RefreshDatabase;

    public function test_public_identity_and_error_layouts_load_the_shared_art_direction(): void
    {
        foreach (['/en', '/login', '/register'] as $path) {
            $this->get($path)->assertOk()
                ->assertSee('css/portal-art-direction.css', false)
                ->assertDontSee('css/app.css', false);
        }

        $this->get('/portal-art-direction-missing-page')->assertNotFound()
            ->assertSee('css/portal-art-direction.css', false);
    }

    public function test_both_home_locales_resolve_the_new_copy_and_icon_partial(): void
    {
        foreach (['en' => 'A persistent world awaits', 'pl' => 'Świat przygód czeka'] as $locale => $copy) {
            $this->get('/'.$locale)->assertOk()
                ->assertSee($copy)
                ->assertSee('class="realm-icon"', false)
                ->assertSee('scene-explore', false)
                ->assertDontSee('portal_art.', false);
        }
    }

    public function test_all_decorative_art_dependencies_are_shipped_without_replacing_admin_styles(): void
    {
        foreach (['css/portal-art-direction.css', 'css/home-production.css', 'images/oteryn-vistas.webp', 'images/oteryn-citadel.webp'] as $path) {
            self::assertFileExists(public_path($path));
        }

        $adminLayout = file_get_contents(resource_path('views/admin/layout.blade.php'));
        self::assertIsString($adminLayout);
        self::assertStringNotContainsString('portal-art-direction.css', $adminLayout);
        self::assertStringContainsString('css/app.css', $adminLayout);
    }
}
