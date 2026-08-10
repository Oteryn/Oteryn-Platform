<?php

namespace Tests\Feature\Downloads;

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class DownloadCenterTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        Config::set('downloads.allowed_artifact_hosts', ['downloads.example.test']);
        Config::set('downloads.immutable_reference_contracts', [
            'downloads.example.test' => [
                'type' => 'object_version_query',
                'parameter' => 'versionId',
            ],
        ]);
    }

    public function test_public_download_center_shows_only_current_published_releases_and_filters_platforms(): void
    {
        $draft = $this->createRelease('9.9.9-draft', DownloadCatalog::CHANNEL_STABLE, null, true);
        $draft->artifacts()->create($this->artifact(
            'https://downloads.example.test/releases/draft-client.zip?versionId=object-version-draft',
            DownloadCatalog::PLATFORM_WINDOWS,
            DownloadCatalog::ARCHITECTURE_X86_64,
            'draft-client.zip',
            str_repeat('d', 64),
        ));

        $stable = $this->createRelease('1.2.3', DownloadCatalog::CHANNEL_STABLE, now()->subMinute(), true);
        $stable->artifacts()->createMany([
            $this->artifact(
                'https://downloads.example.test/releases/1.2.3/oteryn-windows.zip?versionId=object-version-windows-123',
                DownloadCatalog::PLATFORM_WINDOWS,
                DownloadCatalog::ARCHITECTURE_X86_64,
                'oteryn-windows.zip',
                str_repeat('a', 64),
            ),
            $this->artifact(
                'https://downloads.example.test/releases/1.2.3/oteryn-linux.tar.gz?versionId=object-version-linux-123',
                DownloadCatalog::PLATFORM_LINUX,
                DownloadCatalog::ARCHITECTURE_X86_64,
                'oteryn-linux.tar.gz',
                str_repeat('b', 64),
            ),
        ]);

        $beta = $this->createRelease('1.3.0-beta.1', DownloadCatalog::CHANNEL_BETA, now()->subMinute(), true);
        $beta->artifacts()->create($this->artifact(
            'https://downloads.example.test/releases/1.3.0-beta.1/oteryn-beta-windows.zip?versionId=object-version-beta-windows-130',
            DownloadCatalog::PLATFORM_WINDOWS,
            DownloadCatalog::ARCHITECTURE_ARM64,
            'oteryn-beta-windows.zip',
            str_repeat('c', 64),
        ));

        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('Oteryn Client 1.2.3')
            ->assertSeeText('Oteryn Client 1.3.0-beta.1')
            ->assertSeeText('Windows')
            ->assertSeeText('Linux')
            ->assertSeeText('x86-64')
            ->assertSeeText('ARM64')
            ->assertSeeText('oteryn-windows.zip')
            ->assertSeeText('100 MB')
            ->assertSeeText(str_repeat('a', 64))
            ->assertSee('https://downloads.example.test/releases/1.2.3/oteryn-windows.zip?versionId=object-version-windows-123', false)
            ->assertSeeText('does not claim that it independently verified the checksum')
            ->assertDontSeeText('9.9.9-draft')
            ->assertDontSeeText('draft-client.zip');

        $this->get(route('downloads.index', ['platform' => DownloadCatalog::PLATFORM_WINDOWS]))
            ->assertOk()
            ->assertSeeText('oteryn-windows.zip')
            ->assertSeeText('oteryn-beta-windows.zip')
            ->assertDontSeeText('oteryn-linux.tar.gz');

        $this->get(route('downloads.index', ['platform' => DownloadCatalog::PLATFORM_MACOS]))
            ->assertOk()
            ->assertSeeText('No current download is available for macOS.');

        $this->get('/download/android')->assertNotFound();
    }

    public function test_public_download_center_fails_closed_when_a_current_url_is_no_longer_approved(): void
    {
        $release = $this->createRelease('2.0.0', DownloadCatalog::CHANNEL_STABLE, now()->subMinute(), true);
        $release->artifacts()->create($this->artifact(
            'https://downloads.example.test/releases/2.0.0/oteryn-client.zip?versionId=object-version-client-200',
            DownloadCatalog::PLATFORM_WINDOWS,
            DownloadCatalog::ARCHITECTURE_X86_64,
            'oteryn-client.zip',
            str_repeat('e', 64),
        ));

        Config::set('downloads.allowed_artifact_hosts', ['new-downloads.example.test']);

        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('Downloads are temporarily unavailable.')
            ->assertDontSee('https://downloads.example.test/releases/2.0.0/oteryn-client.zip', false);
    }

    public function test_administrator_routes_require_authentication_confirmed_mfa_and_exact_permission(): void
    {
        $this->get(route('admin.downloads.index'))->assertRedirect('/login');

        $withoutMfa = $this->createIdentity('downloads-no-mfa@example.com', false);
        $this->grantDownloadsPermission($withoutMfa);
        $this->actingAsCurrent($withoutMfa);
        $this->get(route('admin.downloads.index'))->assertForbidden();

        $withoutPermission = $this->createIdentity('downloads-no-permission@example.com');
        $this->assignRole($withoutPermission, AdminRoleManager::SECURITY_ADMIN);
        $this->actingAsCurrent($withoutPermission);
        $this->get(route('admin.downloads.index'))->assertForbidden();
    }

    public function test_authorized_administrator_can_create_publish_and_promote_immutable_releases_with_audit(): void
    {
        $actor = $this->createIdentity('downloads-manager@example.com');
        $this->grantDownloadsPermission($actor);
        $this->actingAsCurrent($actor);

        $this->get(route('admin.downloads.create'))
            ->assertOk()
            ->assertSeeText('No executable upload is available.')
            ->assertDontSee('type="file"', false);

        $this->post(route('admin.downloads.store'), $this->releasePayload('3.0.0'))
            ->assertRedirect();

        $first = ClientRelease::query()->where('version', '3.0.0')->firstOrFail();
        self::assertNull($first->published_at);
        self::assertFalse($first->is_current);
        $this->get(route('downloads.index'))->assertDontSeeText('3.0.0');

        $this->post(route('admin.downloads.publish', $first), ['make_current' => true])
            ->assertRedirect(route('admin.downloads.edit', $first));

        $first->refresh();
        self::assertNotNull($first->published_at);
        self::assertTrue($first->is_current);
        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('Oteryn Client 3.0.0')
            ->assertSeeText('oteryn-client-3.0.0.zip');

        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'downloads.release_created',
            'target_type' => 'client_release',
            'target_id' => (string) $first->id,
        ]);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'downloads.release_published',
            'target_type' => 'client_release',
            'target_id' => (string) $first->id,
        ]);

        $this->post(route('admin.downloads.store'), $this->releasePayload('3.1.0'))
            ->assertRedirect();
        $second = ClientRelease::query()->where('version', '3.1.0')->firstOrFail();
        $this->post(route('admin.downloads.publish', $second), ['make_current' => true])
            ->assertRedirect(route('admin.downloads.edit', $second));

        $first->refresh();
        $second->refresh();
        self::assertFalse($first->is_current);
        self::assertTrue($second->is_current);
        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('Oteryn Client 3.1.0')
            ->assertDontSeeText('Oteryn Client 3.0.0');

        $this->put(route('admin.downloads.update', $second), $this->releasePayload('3.1.1'))
            ->assertSessionHasErrors('release');
        $second->refresh();
        self::assertSame('3.1.0', $second->version);
    }

    public function test_administrator_validation_rejects_unsafe_hosts_and_mutable_references(): void
    {
        $actor = $this->createIdentity('downloads-validator@example.com');
        $this->grantDownloadsPermission($actor);
        $this->actingAsCurrent($actor);

        $invalidUrls = [
            'javascript:alert(1)',
            'data:application/octet-stream;base64,AA==',
            'https://evil.example.test/releases/client.zip?versionId=object-version-evil',
            'https://downloads.example.test/latest/client.zip',
            'https://downloads.example.test/releases/3.0.0/client.zip',
            'https://downloads.example.test/releases/3.0.0/client.zip?versionId=latest',
        ];

        foreach ($invalidUrls as $index => $url) {
            $payload = $this->releasePayload('invalid-'.$index);
            $payload['artifacts'][0]['artifact_url'] = $url;

            $this->post(route('admin.downloads.store'), $payload)
                ->assertSessionHasErrors('artifacts.0.artifact_url');
        }

        self::assertSame(0, ClientRelease::query()->where('version', 'like', 'invalid-%')->count());
    }

    public function test_publication_revalidates_the_current_immutable_reference_contract(): void
    {
        $actor = $this->createIdentity('downloads-publisher-revalidation@example.com');
        $this->grantDownloadsPermission($actor);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.downloads.store'), $this->releasePayload('3.2.0'))
            ->assertRedirect();

        $release = ClientRelease::query()->where('version', '3.2.0')->firstOrFail();
        self::assertNull($release->published_at);

        Config::set('downloads.immutable_reference_contracts', []);

        $this->post(route('admin.downloads.publish', $release), ['make_current' => true])
            ->assertSessionHasErrors('artifacts');

        $release->refresh();
        self::assertNull($release->published_at);
        self::assertFalse($release->is_current);
    }

    /**
     * @return array{version: string, channel: string, release_notes: string, artifacts: list<array{platform: string, architecture: string, artifact_url: string, filename: string, size_bytes: int, sha256: string, is_enabled: bool}>}
     */
    private function releasePayload(string $version): array
    {
        $versionId = rawurlencode("object-version-{$version}");

        return [
            'version' => $version,
            'channel' => DownloadCatalog::CHANNEL_STABLE,
            'release_notes' => 'Production test release.',
            'artifacts' => [[
                'platform' => DownloadCatalog::PLATFORM_WINDOWS,
                'architecture' => DownloadCatalog::ARCHITECTURE_X86_64,
                'artifact_url' => "https://downloads.example.test/releases/{$version}/oteryn-client.zip?versionId={$versionId}",
                'filename' => "oteryn-client-{$version}.zip",
                'size_bytes' => 104857600,
                'sha256' => str_repeat('f', 64),
                'is_enabled' => true,
            ]],
        ];
    }

    /**
     * @return array{platform: string, architecture: string, artifact_url: string, filename: string, size_bytes: int, sha256: string, is_enabled: bool}
     */
    private function artifact(
        string $url,
        string $platform,
        string $architecture,
        string $filename,
        string $sha256,
    ): array {
        return [
            'platform' => $platform,
            'architecture' => $architecture,
            'artifact_url' => $url,
            'filename' => $filename,
            'size_bytes' => 104857600,
            'sha256' => $sha256,
            'is_enabled' => true,
        ];
    }

    private function createRelease(
        string $version,
        string $channel,
        ?Carbon $publishedAt,
        bool $isCurrent,
    ): ClientRelease {
        return ClientRelease::query()->create([
            'version' => $version,
            'channel' => $channel,
            'release_notes' => 'Release notes.',
            'published_at' => $publishedAt,
            'is_current' => $isCurrent,
        ]);
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

    private function grantDownloadsPermission(Identity $identity): void
    {
        $roleId = $this->databaseId('admin_roles', AdminRoleManager::CONTENT_EDITOR);
        $permissionId = $this->databaseId('admin_permissions', AdminPermission::MANAGE_DOWNLOADS);

        DB::table('admin_role_permissions')->insertOrIgnore([
            'role_id' => $roleId,
            'permission_id' => $permissionId,
        ]);
        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    private function assignRole(Identity $identity, string $roleKey): void
    {
        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $this->databaseId('admin_roles', $roleKey),
        ]);
    }

    private function databaseId(string $table, string $key): int
    {
        $id = DB::table($table)->where('key', $key)->value('id');

        if (is_int($id)) {
            return $id;
        }

        if (is_string($id) && ctype_digit($id)) {
            return (int) $id;
        }

        self::fail("Expected integer-compatible id for {$table}.{$key}.");
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
