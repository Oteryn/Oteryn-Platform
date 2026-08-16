<?php

namespace Tests\Feature\Downloads;

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Downloads\Actions\ActivateUpdaterGeneration;
use App\Downloads\Actions\ApproveUpdaterPolicy;
use App\Downloads\Actions\EnableUpdaterRelease;
use App\Downloads\Actions\ImportSignedUpdaterGeneration;
use App\Downloads\Actions\WithdrawUpdaterRelease;
use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Closure;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Str;
use Illuminate\Validation\ValidationException;
use Tests\TestCase;

final class UpdaterDistributionTest extends TestCase
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

    public function test_updater_release_identity_allocates_channel_scoped_monotonic_sequence_without_version_ordering(): void
    {
        $actor = $this->administrator('updater-sequence@example.com');
        $lexicallyHigh = $this->release('10.0.0', DownloadCatalog::CHANNEL_STABLE, true);
        $lexicallyLow = $this->release('2.0.0', DownloadCatalog::CHANNEL_STABLE, false);
        $beta = $this->release('99.0.0-beta.1', DownloadCatalog::CHANNEL_BETA, true);

        $first = $this->enable($actor, $lexicallyHigh);
        $second = $this->enable($actor, $lexicallyLow);
        $betaEnabled = $this->enable($actor, $beta);

        self::assertSame(1, $first->updater_sequence);
        self::assertSame(2, $second->updater_sequence);
        self::assertSame(1, $betaEnabled->updater_sequence);
        self::assertNotSame($first->updater_release_id, $second->updater_release_id);
        self::assertTrue($first->is_current);
        self::assertFalse($second->is_current);

        $target = $second->artifacts->firstOrFail()->updater_target_path;
        self::assertIsString($target);
        self::assertStringContainsString('/windows/x86_64/', $target);
        self::assertStringContainsString((string) $second->updater_release_id, $target);

        $idempotent = $this->enable($actor, $lexicallyHigh);
        self::assertSame($first->updater_release_id, $idempotent->updater_release_id);
        self::assertSame(1, $idempotent->updater_sequence);
    }

    public function test_policy_revision_requires_explicit_rollback_preserves_revocation_history_and_respects_withdrawal(): void
    {
        $actor = $this->administrator('updater-policy@example.com');
        $first = $this->enable($actor, $this->release('1.0.0', DownloadCatalog::CHANNEL_STABLE, false));
        $second = $this->enable($actor, $this->release('2.0.0', DownloadCatalog::CHANNEL_STABLE, true));
        $operation = Str::uuid()->toString();

        $policy = $this->approve($actor, $operation, $second, 1, DownloadCatalog::UPDATE_MODE_RECOMMENDED);
        self::assertSame(1, $policy->revision);
        self::assertSame(2, $policy->current_release_sequence);

        $idempotent = $this->approve($actor, $operation, $second, 1, DownloadCatalog::UPDATE_MODE_RECOMMENDED);
        self::assertSame($policy->id, $idempotent->id);

        $this->assertValidationError('operation_id', fn () => $this->approve(
            $actor,
            $operation,
            $second,
            1,
            DownloadCatalog::UPDATE_MODE_REQUIRED,
        ));

        $this->assertValidationError('rollback_authorization', fn () => $this->approve(
            $actor,
            Str::uuid()->toString(),
            $first,
            1,
            DownloadCatalog::UPDATE_MODE_RECOMMENDED,
        ));

        $rollback = $this->approve(
            $actor,
            Str::uuid()->toString(),
            $first,
            1,
            DownloadCatalog::UPDATE_MODE_RECOMMENDED,
            DownloadCatalog::ROLLBACK_EXPLICIT,
        );
        self::assertSame(2, $rollback->revision);
        self::assertSame(1, $rollback->current_release_sequence);

        $targetToRevoke = (string) $first->artifacts->firstOrFail()->updater_target_path;
        $revocation = $this->approve(
            $actor,
            Str::uuid()->toString(),
            $second,
            1,
            DownloadCatalog::UPDATE_MODE_REQUIRED,
            DownloadCatalog::ROLLBACK_NONE,
            [(string) $first->updater_release_id],
            [$targetToRevoke],
        );
        self::assertSame(3, $revocation->revision);
        self::assertSame([(string) $first->updater_release_id], $revocation->revoked_release_ids);
        self::assertSame([$targetToRevoke], $revocation->revoked_artifact_targets);

        $withdrawn = app(WithdrawUpdaterRelease::class)->execute($actor, $first);
        self::assertNotNull($withdrawn->updater_withdrawn_at);
        self::assertFalse($withdrawn->is_current);

        $this->assertValidationError('current_release_id', fn () => $this->approve(
            $actor,
            Str::uuid()->toString(),
            $withdrawn,
            1,
            DownloadCatalog::UPDATE_MODE_OPTIONAL,
            DownloadCatalog::ROLLBACK_EXPLICIT,
        ));
    }

    public function test_protected_generation_import_is_idempotent_and_fails_closed_on_mismatch_staleness_and_replay(): void
    {
        $actor = $this->administrator('updater-import@example.com');
        $release = $this->enable($actor, $this->release('3.0.0', DownloadCatalog::CHANNEL_STABLE, true));
        $policy = $this->approve(
            $actor,
            Str::uuid()->toString(),
            $release,
            1,
            DownloadCatalog::UPDATE_MODE_OPTIONAL,
        );
        $payload = $this->generationPayload($policy, 'stable-generation-1', 1);

        $generation = app(ImportSignedUpdaterGeneration::class)->execute($actor, $payload);
        $same = app(ImportSignedUpdaterGeneration::class)->execute($actor, $payload);
        self::assertSame($generation->id, $same->id);
        self::assertNotNull(DB::table('client_update_generations')->where('id', $generation->id)->value('reconciled_at'));
        self::assertNull($generation->activated_at);

        $freshnessOnly = $this->generationPayload($policy, 'stable-generation-freshness-2', 1);
        $freshnessOnly['timestamp_version'] = 2;
        $freshness = app(ImportSignedUpdaterGeneration::class)->execute($actor, $freshnessOnly);
        self::assertSame(1, $freshness->targets_version);
        self::assertSame(1, $freshness->snapshot_version);
        self::assertSame(2, $freshness->timestamp_version);

        $changedIdentity = $payload;
        $changedIdentity['metadata_set_sha256'] = str_repeat('9', 64);
        $this->assertValidationError('generation_id', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $changedIdentity));

        $wrongPolicyTarget = $this->generationPayload($policy, 'stable-generation-wrong-policy', 3);
        $wrongPolicyTarget['policy_target_sha256'] = str_repeat('8', 64);
        $this->assertValidationError('policy_target', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $wrongPolicyTarget));

        $wrongTarget = $this->generationPayload($policy, 'stable-generation-wrong-target', 3);
        $wrongTarget['targets'][0]['target_path'] .= '.other';
        $this->assertValidationError('targets', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $wrongTarget));

        $wrongChannel = $this->generationPayload($policy, 'beta-generation-mismatch', 3);
        $wrongChannel['channel'] = DownloadCatalog::CHANNEL_BETA;
        $this->assertValidationError('channel', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $wrongChannel));

        $replay = $this->generationPayload($policy, 'stable-generation-replay', 1);
        $this->assertValidationError('metadata_versions', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $replay));

        $expired = $this->generationPayload($policy, 'stable-generation-expired', 3);
        $expired['metadata_expires_at'] = now()->subMinute()->utc()->toIso8601String();
        $this->assertValidationError('metadata_expires_at', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $expired));
    }

    public function test_platform_activation_is_separate_from_browser_publication_and_surfaces_revocation_and_mismatch_truthfully(): void
    {
        $actor = $this->administrator('updater-activation@example.com');
        $release = $this->enable($actor, $this->release('4.0.0', DownloadCatalog::CHANNEL_STABLE, true));
        $policy = $this->approve(
            $actor,
            Str::uuid()->toString(),
            $release,
            1,
            DownloadCatalog::UPDATE_MODE_RECOMMENDED,
        );
        $generation = app(ImportSignedUpdaterGeneration::class)->execute(
            $actor,
            $this->generationPayload($policy, 'stable-generation-active-1', 1),
        );
        $active = app(ActivateUpdaterGeneration::class)->execute($actor, $generation);
        self::assertNotNull($active->activated_at);

        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('Platform-active signed generation selects this exact release')
            ->assertSeeText('The first-party updater independently verifies TUF signatures')
            ->assertSeeText($release->artifacts->firstOrFail()->filename);

        $revokedTarget = (string) $release->artifacts->firstOrFail()->updater_target_path;
        $policyTwo = $this->approve(
            $actor,
            Str::uuid()->toString(),
            $release,
            1,
            DownloadCatalog::UPDATE_MODE_REQUIRED,
            DownloadCatalog::ROLLBACK_NONE,
            [],
            [$revokedTarget],
        );
        $generationTwo = app(ImportSignedUpdaterGeneration::class)->execute(
            $actor,
            $this->generationPayload($policyTwo, 'stable-generation-active-2', 2),
        );
        app(ActivateUpdaterGeneration::class)->execute($actor, $generationTwo);

        $generation->refresh();
        self::assertNotNull($generation->superseded_at);
        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('revokes at least one exact platform/architecture target')
            ->assertSeeText($release->artifacts->firstOrFail()->filename);

        $this->assertValidationError('generation', fn () => app(ActivateUpdaterGeneration::class)
            ->execute($actor, $generation));

        $release->forceFill(['is_current' => false])->save();
        $browserOnly = $this->release('4.1.0', DownloadCatalog::CHANNEL_STABLE, true);
        $this->get(route('downloads.index'))
            ->assertOk()
            ->assertSeeText('Browser current and updater current do not match')
            ->assertSeeText($browserOnly->artifacts->firstOrFail()->filename)
            ->assertDontSeeText('Oteryn Client 4.0.0');
    }

    public function test_signed_generation_boundary_rejects_secret_shaped_fields_and_has_no_web_admin_mutation_route(): void
    {
        $actor = $this->administrator('updater-private-key@example.com');
        $release = $this->enable($actor, $this->release('5.0.0', DownloadCatalog::CHANNEL_STABLE, true));
        $policy = $this->approve(
            $actor,
            Str::uuid()->toString(),
            $release,
            1,
            DownloadCatalog::UPDATE_MODE_OPTIONAL,
        );
        $payload = $this->generationPayload($policy, 'stable-generation-private-key-reject', 1);
        $payload['private_key'] = 'must-never-be-accepted';

        $this->assertValidationError('generation_payload', fn () => app(ImportSignedUpdaterGeneration::class)
            ->execute($actor, $payload));
        self::assertSame(0, ClientUpdateGeneration::query()->count());

        $this->actingAsCurrent($actor);
        $this->post('/admin/downloads/updater/stable/generations', [
            'public_metadata_json' => '{}',
        ])->assertNotFound();
        $this->post('/admin/downloads/updater/stable/generations/1/activate')->assertNotFound();
        $this->get(route('admin.downloads.updater', ['channel' => 'stable']))
            ->assertOk()
            ->assertSeeText('This web console intentionally has no route to import or activate signed-generation metadata.')
            ->assertDontSee('public_metadata_json', false)
            ->assertDontSeeText('Activate Platform updater state');

        $columns = array_merge(
            Schema::getColumnListing('client_update_policies'),
            Schema::getColumnListing('client_update_generations'),
            Schema::getColumnListing('client_releases'),
        );
        foreach ($columns as $column) {
            self::assertIsString($column);
            self::assertStringNotContainsString('private_key', $column);
            self::assertStringNotContainsString('signing_secret', $column);
        }
    }

    private function enable(Identity $actor, ClientRelease $release): ClientRelease
    {
        return app(EnableUpdaterRelease::class)->execute($actor, $release);
    }

    /**
     * @param  list<string>  $revokedReleaseIds
     * @param  list<string>  $revokedArtifactTargets
     */
    private function approve(
        Identity $actor,
        string $operationId,
        ClientRelease $release,
        int $minimumSequence,
        string $mode,
        string $rollback = DownloadCatalog::ROLLBACK_NONE,
        array $revokedReleaseIds = [],
        array $revokedArtifactTargets = [],
    ): ClientUpdatePolicy {
        return app(ApproveUpdaterPolicy::class)->execute(
            $actor,
            $operationId,
            $release->channel,
            $release->id,
            $minimumSequence,
            $mode,
            $rollback,
            $revokedReleaseIds,
            $revokedArtifactTargets,
        );
    }

    /**
     * @return array{generation_id: string, channel: string, policy_revision: int, root_version: int, targets_version: int, snapshot_version: int, timestamp_version: int, metadata_expires_at: string, metadata_set_sha256: string, policy_target_path: string, policy_target_sha256: string, policy_target_length: int, targets: list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}>}
     */
    private function generationPayload(ClientUpdatePolicy $policy, string $generationId, int $version): array
    {
        /** @var list<array{artifact_id: int, platform: string, architecture: string, target_path: string, size_bytes: int, supplied_sha256: string}> $artifactTargets */
        $artifactTargets = $policy->artifact_targets;
        $targets = array_map(
            static fn (array $target): array => [
                'platform' => $target['platform'],
                'architecture' => $target['architecture'],
                'target_path' => $target['target_path'],
                'length' => $target['size_bytes'],
                'sha256' => $target['supplied_sha256'],
            ],
            $artifactTargets,
        );

        return [
            'generation_id' => $generationId,
            'channel' => $policy->channel,
            'policy_revision' => $policy->revision,
            'root_version' => 1,
            'targets_version' => $version,
            'snapshot_version' => $version,
            'timestamp_version' => $version,
            'metadata_expires_at' => now()->addHour()->utc()->toIso8601String(),
            'metadata_set_sha256' => hash('sha256', 'public-metadata|'.$generationId),
            'policy_target_path' => $policy->policy_target_path,
            'policy_target_sha256' => $policy->policy_document_sha256,
            'policy_target_length' => $policy->policy_document_length,
            'targets' => $targets,
        ];
    }

    private function release(string $version, string $channel, bool $isCurrent): ClientRelease
    {
        $release = ClientRelease::query()->create([
            'version' => $version,
            'channel' => $channel,
            'release_notes' => "Release {$version}.",
            'published_at' => now()->subMinute(),
            'is_current' => $isCurrent,
        ]);
        $versionId = rawurlencode('object-version-'.$version);
        $release->artifacts()->create([
            'platform' => DownloadCatalog::PLATFORM_WINDOWS,
            'architecture' => DownloadCatalog::ARCHITECTURE_X86_64,
            'artifact_url' => "https://downloads.example.test/releases/{$version}/client.zip?versionId={$versionId}",
            'filename' => 'oteryn-'.$version.'.zip',
            'size_bytes' => 1_572_864,
            'sha256' => hash('sha256', 'artifact|'.$version.'|'.$channel),
            'is_enabled' => true,
        ]);

        return $release->load('artifacts');
    }

    private function administrator(string $email): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();

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

        return $identity;
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

    private function assertValidationError(string $key, Closure $operation): void
    {
        try {
            $operation();
            self::fail("Expected validation error for {$key}.");
        } catch (ValidationException $exception) {
            self::assertArrayHasKey($key, $exception->errors());
        }
    }
}
