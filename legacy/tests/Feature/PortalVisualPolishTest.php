<?php

namespace Tests\Feature;

use App\Admin\AdminAuthorization;
use App\Cms\Models\NewsPost;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\Wiki\ViewModels\Public\WikiArticleCard;
use App\Wiki\ViewModels\Public\WikiHomeViewModel;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class PortalVisualPolishTest extends TestCase
{
    use RefreshDatabase;

    public function test_home_uses_sized_lazy_discovery_images_without_duplicate_legacy_heading(): void
    {
        foreach (['en', 'pl'] as $locale) {
            $response = $this->get('/'.$locale)->assertOk();
            $response->assertSee('width="240" height="100" loading="lazy" decoding="async"', false)
                ->assertDontSee('realm-hero-call', false)
                ->assertDontSee('class="journal-illustration', false)
                ->assertSee('id="home-hero-title"', false);
            $html = $response->getContent();
            self::assertIsString($html);
            self::assertSame(6, substr_count($html, 'class="discovery-illustration'));
        }
    }

    public function test_public_portal_assets_are_content_versioned_for_cache_safe_delivery(): void
    {
        foreach (['en', 'pl'] as $locale) {
            $response = $this->get('/'.$locale)->assertOk();
            $html = $response->getContent();
            self::assertIsString($html);
            self::assertMatchesRegularExpression('/data-portal-assets="[0-9a-f]{12}"/', $html);

            foreach ([
                'css/portal-system.css',
                'css/portal-pages.css',
                'css/portal-art-direction.css',
                'css/portal-owner-visible.css',
                'css/home-production.css',
                'js/portal-navigation.js',
                'images/oteryn-citadel.webp',
            ] as $path) {
                self::assertMatchesRegularExpression(
                    '~(?:href|src)="[^"]*/'.preg_quote($path, '~').'\?v=[0-9a-f]{12}"~',
                    $html,
                    'Expected a content-versioned first-party asset URL for '.$path,
                );
            }
        }
    }

    public function test_news_uses_one_sized_decorative_lead_and_preserves_real_content(): void
    {
        foreach (['first', 'second', 'third'] as $index => $slug) {
            NewsPost::query()->create([
                'slug' => $slug,
                'title' => 'Chronicle '.$slug,
                'body' => 'Actual published content '.$slug,
                'published_at' => now()->subMinutes($index + 1),
            ]);
        }
        $response = $this->get('/en/news')->assertOk();
        $html = $response->getContent();
        self::assertIsString($html);
        self::assertSame(1, substr_count($html, 'class="chronicle-illustration"'));
        $response->assertSee('width="626" height="468"', false)->assertSeeText('Chronicle second');
        $this->get('/en/news/first')->assertOk()
            ->assertSeeText('Actual published content first')
            ->assertDontSee('reading-world', false);
    }

    public function test_wiki_deduplicates_cards_without_losing_unique_recent_articles(): void
    {
        $featured = new WikiArticleCard(1, 'Featured guide', 'featured-guide', 'Featured summary', now());
        $recent = new WikiArticleCard(2, 'Recent guide', 'recent-guide', 'Recent summary', now());
        $wiki = new WikiHomeViewModel([], [$featured], [$featured, $recent]);
        $html = view('wiki.index', compact('wiki'))->render();
        self::assertSame(1, substr_count($html, 'Featured summary'));
        self::assertSame(1, substr_count($html, 'Recent summary'));
        self::assertStringContainsString('id="wiki-recent"', $html);

        $wiki = new WikiHomeViewModel([], [$featured], [$featured]);
        $sparseHtml = view('wiki.index', compact('wiki'))->render();
        self::assertStringNotContainsString('id="wiki-recent"', $sparseHtml);
        self::assertSame(1, substr_count($sparseHtml, 'Featured summary'));
    }

    public function test_admin_task_directory_does_not_offer_unauthorized_operations(): void
    {
        $identity = Identity::query()->create([
            'email' => 'visual-editor@example.test',
            'password' => Hash::make('Isolated-Visual-Test!234'),
        ]);
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();
        $identity->refresh();
        $role = DB::table('admin_roles')->where('key', 'content_editor')->value('id');
        self::assertNotNull($role);
        DB::table('identity_admin_roles')->insert(['identity_id' => $identity->id, 'role_id' => $role]);
        $this->actingAs($identity, 'web')->withSession([
            WebSessionState::GENERATION_KEY => $identity->web_session_generation,
        ]);
        $this->get('/admin')->assertOk()
            ->assertSee('admin-task-group-content', false)
            ->assertDontSee('admin-task-group-access', false)
            ->assertDontSee('admin-task-group-operations', false);
        $this->get('/admin/roles')->assertForbidden();
        $permissions = app(AdminAuthorization::class)->grantedPermissions($identity);
        self::assertContains('cms.news.manage', $permissions);
        self::assertNotContains('admin.roles.manage', $permissions);
        DB::table('identity_admin_roles')->where('identity_id', $identity->id)->delete();
        self::assertSame([], app(AdminAuthorization::class)->grantedPermissions($identity));
        $this->get('/admin')->assertForbidden();
    }
}
