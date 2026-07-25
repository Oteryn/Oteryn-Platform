<?php

namespace Tests\Feature\Localization;

use App\Admin\AdminRoleManager;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialTranslationResolver;
use App\Cms\Editorial\EditorialTranslationState;
use App\Cms\Models\EditorialTranslation;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\Localization\LocaleFormatter;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class PublicLocalizationTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('downloads.allowed_artifact_hosts', ['downloads.example.test']);
    }

    public function test_public_routes_are_deterministic_and_legacy_bookmarks_use_english_compatibility(): void
    {
        $this->get('/en/news')
            ->assertOk()
            ->assertHeader('Content-Language', 'en')
            ->assertSee('<link rel="canonical" href="'.url('/en/news').'">', false);

        $this->get('/pl/news')
            ->assertOk()
            ->assertHeader('Content-Language', 'pl')
            ->assertSeeText('Aktualności')
            ->assertSeeText('Język: Polski');

        $this->get('/news')
            ->assertOk()
            ->assertHeader('Content-Language', 'en')
            ->assertSee('<link rel="canonical" href="'.url('/en/news').'">', false);

        self::assertStringEndsWith('/en/news', route('news.index'));
        self::assertStringEndsWith('/pl/news', route('news.index', ['locale' => 'pl']));
    }

    public function test_only_the_legacy_home_page_negotiates_locale_with_explicit_precedence(): void
    {
        $this->withHeader('Accept-Language', 'pl-PL,pl;q=0.9,en;q=0.5')
            ->get('/')
            ->assertOk()
            ->assertHeader('Content-Language', 'pl')
            ->assertSeeText('Odpowiedz na wezwanie Oteryn');

        $this->withCookie('oteryn_locale', 'pl')
            ->withHeader('Accept-Language', 'en')
            ->get('/')
            ->assertOk()
            ->assertHeader('Content-Language', 'pl');

        $this->withCookie('oteryn_locale', 'pl')
            ->withHeader('Accept-Language', 'pl')
            ->get('/?lang=en')
            ->assertOk()
            ->assertHeader('Content-Language', 'en');

        $this->withHeader('Accept-Language', 'pl-PL')
            ->get('/news')
            ->assertOk()
            ->assertHeader('Content-Language', 'en');
    }

    public function test_polish_news_is_not_published_without_a_fresh_explicit_translation(): void
    {
        $post = NewsPost::query()->create([
            'slug' => 'source-update',
            'title' => 'English source title',
            'body' => 'English source body',
            'published_at' => now()->subMinute(),
        ]);

        $this->get('/en/news/source-update')
            ->assertOk()
            ->assertSeeText('English source title');

        $this->get('/pl/news/source-update')
            ->assertNotFound()
            ->assertDontSeeText('English source title');

        EditorialTranslation::query()->create([
            'content_type' => EditorialContentType::NewsPost->value,
            'content_id' => $post->id,
            'locale' => 'pl',
            'title' => 'Polski tytuł',
            'body' => 'Polska treść',
            'source_updated_at' => $post->updated_at,
            'published_at' => now()->subSecond(),
        ]);

        $this->get('/pl/news/source-update')
            ->assertOk()
            ->assertSeeText('Polski tytuł')
            ->assertSeeText('Polska treść')
            ->assertDontSeeText('English source title');

        $this->get('/en/news/source-update')
            ->assertOk()
            ->assertSee('hreflang="pl" href="'.url('/pl/news/source-update').'"', false);

        $this->travel(2)->seconds();
        $post->forceFill(['body' => 'Changed English source'])->save();

        $this->get('/pl/news/source-update')
            ->assertNotFound()
            ->assertDontSeeText('Changed English source');

        $this->get('/en/news/source-update')
            ->assertOk()
            ->assertDontSee(url('/pl/news/source-update'), false);

        self::assertSame(
            EditorialTranslationState::Stale,
            app(EditorialTranslationResolver::class)->state(
                EditorialContentType::NewsPost,
                $post->id,
                $post->updated_at,
                'pl',
            ),
        );
    }

    public function test_editorial_translation_states_are_explicit_and_incomplete_content_never_publishes(): void
    {
        $post = NewsPost::query()->create([
            'slug' => 'translation-states',
            'title' => 'Source',
            'body' => 'Source body',
            'published_at' => now()->subMinute(),
        ]);
        $resolver = app(EditorialTranslationResolver::class);

        self::assertSame(
            EditorialTranslationState::Missing,
            $resolver->state(EditorialContentType::NewsPost, $post->id, $post->updated_at, 'pl'),
        );

        $translation = EditorialTranslation::query()->create([
            'content_type' => EditorialContentType::NewsPost->value,
            'content_id' => $post->id,
            'locale' => 'pl',
            'title' => 'Tytuł',
            'body' => null,
            'source_updated_at' => $post->updated_at,
            'published_at' => now()->subSecond(),
        ]);

        self::assertSame(
            EditorialTranslationState::Incomplete,
            $resolver->state(EditorialContentType::NewsPost, $post->id, $post->updated_at, 'pl'),
        );
        $this->get('/pl/news/translation-states')->assertNotFound();

        $translation->forceFill(['body' => 'Treść', 'published_at' => null])->save();
        self::assertSame(
            EditorialTranslationState::Draft,
            $resolver->state(EditorialContentType::NewsPost, $post->id, $post->updated_at, 'pl'),
        );

        $translation->forceFill(['published_at' => now()->subSecond()])->save();
        self::assertSame(
            EditorialTranslationState::Published,
            $resolver->state(EditorialContentType::NewsPost, $post->id, $post->updated_at, 'pl'),
        );
    }

    public function test_editorial_translation_workflow_requires_mfa_exact_permission_completeness_and_writes_bounded_audit(): void
    {
        $post = NewsPost::query()->create([
            'slug' => 'translated-by-editor',
            'title' => 'English source',
            'body' => 'English body',
            'published_at' => now()->subMinute(),
        ]);

        $guestUrl = route('admin.news.translation.edit', $post);
        $this->get($guestUrl)->assertRedirect('/login');

        $withoutMfa = $this->createIdentity('translation-no-mfa@example.test', false);
        $this->assignRole($withoutMfa, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($withoutMfa);
        $this->get($guestUrl)->assertForbidden();

        $withoutPermission = $this->createIdentity('translation-no-permission@example.test');
        $this->assignRole($withoutPermission, AdminRoleManager::SECURITY_ADMIN);
        $this->actingAsCurrent($withoutPermission);
        $this->get($guestUrl)->assertForbidden();

        $editor = $this->createIdentity('translation-editor@example.test');
        $this->assignRole($editor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($editor);

        $this->get($guestUrl)
            ->assertOk()
            ->assertSeeText('State: missing')
            ->assertSeeText('English source');

        $this->put(route('admin.news.translation.update', $post), [
            'title' => 'Polski tytuł',
            'body' => '',
            'published_at' => now()->subMinute()->format('Y-m-d\TH:i'),
        ])->assertSessionHasErrors('published_at');

        $this->assertDatabaseMissing('editorial_translations', [
            'content_type' => EditorialContentType::NewsPost->value,
            'content_id' => $post->id,
            'locale' => 'pl',
        ]);

        $this->put(route('admin.news.translation.update', $post), [
            'title' => 'Polski tytuł',
            'body' => 'Polska treść',
            'published_at' => now()->subMinute()->format('Y-m-d\TH:i'),
        ])->assertRedirect(route('admin.news.translation.edit', $post));

        $this->assertDatabaseHas('editorial_translations', [
            'content_type' => EditorialContentType::NewsPost->value,
            'content_id' => $post->id,
            'locale' => 'pl',
            'title' => 'Polski tytuł',
            'body' => 'Polska treść',
        ]);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $editor->id,
            'action' => 'cms.translation_saved',
            'target_type' => EditorialContentType::NewsPost->value,
            'target_id' => (string) $post->id,
        ]);
        $this->get('/pl/news/translated-by-editor')
            ->assertOk()
            ->assertSeeText('Polski tytuł');
    }

    public function test_translation_routes_do_not_cross_reserved_managed_page_permission_boundaries(): void
    {
        $reserved = ManagedPage::query()->create([
            'slug' => 'rules',
            'title' => 'Rules',
            'body' => 'Rules body',
            'published_at' => now()->subMinute(),
        ]);
        $generic = ManagedPage::query()->create([
            'slug' => 'generic-information',
            'title' => 'Generic',
            'body' => 'Generic body',
            'published_at' => now()->subMinute(),
        ]);

        $contentEditor = $this->createIdentity('reserved-page-editor@example.test');
        $this->assignRole($contentEditor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($contentEditor);
        $this->get(route('admin.pages.translation.edit', $reserved))->assertNotFound();

        $platformAdmin = $this->createIdentity('support-page-admin@example.test');
        $this->assignRole($platformAdmin, AdminRoleManager::PLATFORM_ADMIN);
        $this->actingAsCurrent($platformAdmin);
        $this->get(route('admin.support-content.translation.edit', $generic))->assertNotFound();
        $this->get(route('admin.support-content.translation.edit', $reserved))->assertOk();
    }

    public function test_event_language_switcher_preserves_the_equivalent_localized_slug(): void
    {
        $event = Event::query()->create([
            'status' => Event::STATUS_SCHEDULED,
            'starts_at' => now()->addDay(),
            'ends_at' => now()->addDays(2),
            'featured' => true,
            'lock_version' => 1,
        ]);
        EventTranslation::query()->create([
            'event_id' => $event->id,
            'locale' => 'en',
            'title' => 'Summer Trial',
            'slug' => 'summer-trial',
            'summary' => 'English summary.',
            'body' => 'English body.',
        ]);
        EventTranslation::query()->create([
            'event_id' => $event->id,
            'locale' => 'pl',
            'title' => 'Letnia Próba',
            'slug' => 'letnia-proba',
            'summary' => 'Polskie podsumowanie.',
            'body' => 'Polska treść.',
        ]);

        $this->get('/en/events/summer-trial')
            ->assertOk()
            ->assertSee('hreflang="pl" href="'.url('/pl/events/letnia-proba').'"', false);

        $this->get('/pl/events/letnia-proba')
            ->assertOk()
            ->assertSeeText('Letnia Próba')
            ->assertSee('hreflang="en" href="'.url('/en/events/summer-trial').'"', false);
    }

    public function test_polish_download_keeps_approved_artifacts_but_never_falls_back_to_english_release_notes(): void
    {
        $release = ClientRelease::query()->create([
            'version' => '4.0.0',
            'channel' => DownloadCatalog::CHANNEL_STABLE,
            'release_notes' => 'English release notes must not appear as Polish.',
            'published_at' => now()->subMinute(),
            'is_current' => true,
        ]);
        $release->artifacts()->create([
            'platform' => DownloadCatalog::PLATFORM_WINDOWS,
            'architecture' => DownloadCatalog::ARCHITECTURE_X86_64,
            'artifact_url' => 'https://downloads.example.test/releases/4.0.0/oteryn.zip',
            'filename' => 'oteryn.zip',
            'size_bytes' => 1_572_864,
            'sha256' => str_repeat('a', 64),
            'is_enabled' => true,
        ]);

        $this->get('/pl/download')
            ->assertOk()
            ->assertSeeText('oteryn.zip')
            ->assertSeeText('1,5 MB')
            ->assertSeeText('Opis wydania nie jest dostępny w tym języku.')
            ->assertDontSeeText('English release notes must not appear as Polish.');

        EditorialTranslation::query()->create([
            'content_type' => EditorialContentType::ClientRelease->value,
            'content_id' => $release->id,
            'locale' => 'pl',
            'title' => null,
            'body' => 'Polskie informacje o wydaniu.',
            'source_updated_at' => $release->updated_at,
            'published_at' => now()->subSecond(),
        ]);

        $this->get('/pl/download')
            ->assertOk()
            ->assertSeeText('Polskie informacje o wydaniu.')
            ->assertDontSeeText('English release notes must not appear as Polish.')
            ->assertDontSeeText('Opis wydania nie jest dostępny w tym języku.');
    }

    public function test_dates_numbers_and_localized_error_states_use_the_active_locale(): void
    {
        $formatter = app(LocaleFormatter::class);
        $value = now()->setDate(2026, 7, 25)->setTime(13, 45);

        self::assertSame('July 25, 2026', $formatter->date($value, 'en'));
        self::assertSame('25 lipca 2026', $formatter->date($value, 'pl'));
        self::assertSame('12,345.67', $formatter->number(12345.67, 2, 'en'));
        self::assertSame("12\u{00A0}345,67", $formatter->number(12345.67, 2, 'pl'));
        self::assertSame('1.5 MB', $formatter->bytes(1_572_864, 'en'));
        self::assertSame('1,5 MB', $formatter->bytes(1_572_864, 'pl'));

        $this->get('/pl/nie-istnieje')
            ->assertNotFound()
            ->assertHeader('Content-Language', 'pl')
            ->assertSeeText('Nie udało się znaleźć tej strony')
            ->assertDontSeeText('We could not find that page');

        $this->get('/en/not-found')
            ->assertNotFound()
            ->assertHeader('Content-Language', 'en')
            ->assertSeeText('We could not find that page');
    }

    private function createIdentity(string $email, bool $confirmedMfa = true): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($confirmedMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    private function assignRole(Identity $identity, string $roleKey): void
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if (! is_int($roleId) && ! (is_string($roleId) && ctype_digit($roleId))) {
            self::fail('Expected an integer-compatible administrator role id.');
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => (int) $roleId,
        ]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
