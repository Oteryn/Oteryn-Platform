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
        foreach (['en' => 'A persistent world awaits', 'pl' => 'Świat pełen przygód czeka'] as $locale => $copy) {
            $this->get('/'.$locale)->assertOk()
                ->assertSee($copy)
                ->assertSee('class="realm-icon"', false)
                ->assertSee('scene-explore', false)
                ->assertDontSee('portal_art.', false);
        }
    }

    public function test_all_decorative_art_dependencies_are_shipped_and_admin_keeps_its_foundation(): void
    {
        foreach (['css/portal-art-direction.css', 'css/home-production.css', 'css/portal-admin.css', 'images/oteryn-vistas.webp', 'images/oteryn-citadel.webp'] as $path) {
            self::assertFileExists(public_path($path));
        }
        foreach (['explore', 'fight', 'market', 'community', 'library', 'chronicles'] as $scene) {
            self::assertFileExists(public_path('images/oteryn-'.$scene.'.webp'));
        }

        $adminLayout = file_get_contents(resource_path('views/admin/layout.blade.php'));
        self::assertIsString($adminLayout);
        self::assertStringContainsString('portal-art-direction.css', $adminLayout);
        self::assertStringContainsString('portal-admin.css', $adminLayout);
        self::assertStringContainsString('css/app.css', $adminLayout);
    }
}
