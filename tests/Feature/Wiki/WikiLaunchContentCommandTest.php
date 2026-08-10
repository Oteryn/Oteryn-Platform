<?php

namespace Tests\Feature\Wiki;

use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use App\Wiki\Content\WikiExpectedContentInventory;
use App\Wiki\Content\WikiLaunchContentCatalog;
use App\Wiki\Domain\WikiArticleStatus;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Testing\PendingCommand;
use Tests\TestCase;

final class WikiLaunchContentCommandTest extends TestCase
{
    use RefreshDatabase;

    public function test_expected_inventory_validates_without_database_mutation(): void
    {
        $command = $this->artisan('wiki:launch-content:validate', [
            '--json' => true,
        ]);
        self::assertInstanceOf(PendingCommand::class, $command);
        $command
            ->expectsOutputToContain('"status":"PASS"')
            ->expectsOutputToContain('"inventory_version":"'.WikiExpectedContentInventory::VERSION.'"')
            ->expectsOutputToContain('"categories":4')
            ->expectsOutputToContain('"articles":13')
            ->assertSuccessful();

        self::assertSame(0, DB::table('wiki_categories')->count());
        self::assertSame(0, DB::table('wiki_articles')->count());
        self::assertSame(0, DB::table('admin_audit_events')->count());
    }

    public function test_exact_mfa_confirmed_publisher_installs_public_content_once(): void
    {
        $publisher = $this->createPublisher(
            'launch-publisher@example.com',
            $this->wikiPermissions(),
        );

        $this->installCommand(' Launch-Publisher@Example.com ')
            ->expectsOutputToContain('Installed Wiki launch content')
            ->assertSuccessful();

        self::assertSame(4, DB::table('wiki_categories')->count());
        self::assertSame(8, DB::table('wiki_category_translations')->count());
        self::assertSame(13, DB::table('wiki_articles')->count());
        self::assertSame(26, DB::table('wiki_article_translations')->count());
        self::assertSame(26, DB::table('wiki_revisions')->count());
        self::assertSame(56, DB::table('admin_audit_events')->count());
        self::assertSame(
            13,
            DB::table('wiki_articles')
                ->where('status', WikiArticleStatus::PUBLISHED->value)
                ->where('publisher_identity_id', $publisher->id)
                ->whereNotNull('published_at')
                ->count(),
        );

        $this->get('/en/wiki/download-and-installation')
            ->assertOk()
            ->assertSeeText('Download from the approved source')
            ->assertSee('/download', false);
        $this->get('/pl/wiki/pobieranie-i-instalacja')
            ->assertOk()
            ->assertSeeText('Pobierz z zatwierdzonego źródła')
            ->assertSee('/download', false);
        $this->get('/en/wiki/server-rates')
            ->assertOk()
            ->assertSeeText('contains no approved numeric');
        $this->get('/pl/wiki/tempo-serwera')
            ->assertOk()
            ->assertSeeText('nie zawiera zatwierdzonych liczbowych');

        $counts = [
            'categories' => DB::table('wiki_categories')->count(),
            'articles' => DB::table('wiki_articles')->count(),
            'revisions' => DB::table('wiki_revisions')->count(),
            'audit' => DB::table('admin_audit_events')->count(),
        ];

        $this->installCommand($publisher->email)
            ->expectsOutputToContain('is already installed exactly')
            ->assertSuccessful();

        self::assertSame($counts['categories'], DB::table('wiki_categories')->count());
        self::assertSame($counts['articles'], DB::table('wiki_articles')->count());
        self::assertSame($counts['revisions'], DB::table('wiki_revisions')->count());
        self::assertSame($counts['audit'], DB::table('admin_audit_events')->count());
    }

    public function test_version_confirmation_is_required_before_any_lookup_or_write(): void
    {
        $command = $this->artisan('wiki:launch-content:install', [
            'publisher' => 'missing@example.com',
        ]);
        self::assertInstanceOf(PendingCommand::class, $command);
        $command
            ->expectsOutputToContain('--content-version='.WikiLaunchContentCatalog::VERSION)
            ->assertFailed();

        self::assertSame(0, DB::table('wiki_categories')->count());
        self::assertSame(0, DB::table('wiki_articles')->count());
        self::assertSame(0, DB::table('admin_audit_events')->count());
    }

    public function test_mfa_and_every_exact_permission_are_required_before_writes(): void
    {
        $withoutMfa = $this->createPublisher(
            'launch-no-mfa@example.com',
            $this->wikiPermissions(),
            false,
        );

        $this->installCommand($withoutMfa->email)
            ->expectsOutputToContain('enabled MFA-confirmed publisher Identity')
            ->assertFailed();

        $withoutPublish = $this->createPublisher(
            'launch-no-publish@example.com',
            [
                AdminPermission::WIKI_ACCESS,
                AdminPermission::MANAGE_WIKI_CATEGORIES,
                AdminPermission::MANAGE_WIKI_ARTICLES,
            ],
        );

        $this->installCommand($withoutPublish->email)
            ->expectsOutputToContain('required Wiki permission')
            ->assertFailed();

        self::assertSame(0, DB::table('wiki_categories')->count());
        self::assertSame(0, DB::table('wiki_articles')->count());
        self::assertSame(0, DB::table('wiki_revisions')->count());
        self::assertSame(0, DB::table('admin_audit_events')->count());
    }

    public function test_conflicting_editorial_content_aborts_without_partial_installation(): void
    {
        $publisher = $this->createPublisher(
            'launch-conflict@example.com',
            $this->wikiPermissions(),
        );
        $now = now();
        $categoryId = DB::table('wiki_categories')->insertGetId([
            'key' => 'getting-started',
            'sort_order' => 999,
            'visible' => true,
            'lock_version' => 1,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
        DB::table('wiki_category_translations')->insert([
            [
                'category_id' => $categoryId,
                'locale' => 'en',
                'name' => 'Editorial content',
                'slug' => 'editorial-content',
                'description' => 'Must never be overwritten.',
                'created_at' => $now,
                'updated_at' => $now,
            ],
            [
                'category_id' => $categoryId,
                'locale' => 'pl',
                'name' => 'Treść redakcyjna',
                'slug' => 'tresc-redakcyjna',
                'description' => 'Nie wolno jej nadpisać.',
                'created_at' => $now,
                'updated_at' => $now,
            ],
        ]);

        $this->installCommand($publisher->email)
            ->expectsOutputToContain('conflicts with reviewed content')
            ->assertFailed();

        self::assertSame(1, DB::table('wiki_categories')->count());
        self::assertSame(0, DB::table('wiki_articles')->count());
        self::assertSame(0, DB::table('wiki_revisions')->count());
        self::assertSame(0, DB::table('admin_audit_events')->count());
        self::assertSame(
            999,
            DB::table('wiki_categories')->where('id', $categoryId)->value('sort_order'),
        );
        self::assertSame(
            'Must never be overwritten.',
            DB::table('wiki_category_translations')
                ->where('category_id', $categoryId)
                ->where('locale', 'en')
                ->value('description'),
        );
    }

    /**
     * @param  list<string>  $permissions
     */
    private function createPublisher(
        string $email,
        array $permissions,
        bool $confirmedMfa = true,
    ): Identity {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($confirmedMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'JBSWY3DPEHPK3PXP',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'wiki-launch-'.$identity->id,
            'name' => 'Wiki launch publisher test',
            'created_at' => now(),
            'updated_at' => now(),
        ]);

        foreach ($permissions as $permission) {
            $permissionId = DB::table('admin_permissions')
                ->where('key', $permission)
                ->value('id');
            self::assertIsInt($permissionId);

            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $permissionId,
            ]);
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);

        return $identity->refresh();
    }

    /**
     * @return list<string>
     */
    private function wikiPermissions(): array
    {
        return [
            AdminPermission::WIKI_ACCESS,
            AdminPermission::MANAGE_WIKI_CATEGORIES,
            AdminPermission::MANAGE_WIKI_ARTICLES,
            AdminPermission::PUBLISH_WIKI,
        ];
    }

    private function installCommand(string $publisher): PendingCommand
    {
        $command = $this->artisan('wiki:launch-content:install', [
            'publisher' => $publisher,
            '--content-version' => WikiLaunchContentCatalog::VERSION,
        ]);
        self::assertInstanceOf(PendingCommand::class, $command);

        return $command;
    }
}
