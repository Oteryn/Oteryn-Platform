<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
use App\Identity\Models\Identity;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;
use Throwable;

final readonly class ImportSignedUpdaterGeneration
{
    private const TOP_LEVEL_KEYS = [
        'generation_id',
        'channel',
        'policy_revision',
        'root_version',
        'targets_version',
        'snapshot_version',
        'timestamp_version',
        'metadata_expires_at',
        'metadata_set_sha256',
        'policy_target_path',
        'policy_target_sha256',
        'policy_target_length',
        'targets',
    ];

    private const TARGET_KEYS = ['platform', 'architecture', 'target_path', 'length', 'sha256'];

    public function __construct(private AdminAuditRecorder $audit) {}

    /**
     * Reconciles the bounded public projection supplied by the protected release-publishing
     * integration after that boundary has verified the required TUF signatures and repository
     * coherence. This action verifies exact Platform policy/target association, freshness and
     * monotonic public metadata identity; it does not cryptographically verify TUF signatures,
     * accept private keys, or replace the first-party updater's trusted TUF verification.
     *
     * There is intentionally no ordinary web-administrator route to this action.
     *
     * @param array<string, mixed> $payload
     */
    public function execute(Identity $actor, array $payload): ClientUpdateGeneration
    {
        $payload = $this->validatedPayload($payload);

        return DB::transaction(function () use ($actor, $payload): ClientUpdateGeneration {
            $expiresAt = Carbon::parse($payload['metadata_expires_at'])->utc();
            $targets = $this->normalizeTargets($payload['targets']);

            $policy = ClientUpdatePolicy::query()
                ->where('channel', $payload['channel'])
                ->where('revision', $payload['policy_revision'])
                ->lockForUpdate()
                ->first();
            if (! $policy instanceof ClientUpdatePolicy) {
                throw ValidationException::withMessages([
                    'channel' => 'The protected-integration generation does not match an approved policy revision in this channel.',
                ]);
            }

            $existing = ClientUpdateGeneration::query()
                ->where('generation_id', $payload['generation_id'])
                ->first();
            if ($existing instanceof ClientUpdateGeneration) {
                $this->assertExistingGeneration($existing, $payload, $expiresAt, $targets);

                return $existing->load('policy');
            }

            $latestPolicy = ClientUpdatePolicy::query()
                ->where('channel', $payload['channel'])
                ->orderByDesc('revision')
                ->lockForUpdate()
                ->first();
            if (! $latestPolicy instanceof ClientUpdatePolicy || $latestPolicy->id !== $policy->id) {
                throw ValidationException::withMessages([
                    'policy_revision' => 'Stale policy generations cannot be imported after a newer policy revision is approved.',
                ]);
            }

            if (! $expiresAt->isFuture()) {
                throw ValidationException::withMessages([
                    'metadata_expires_at' => 'Expired public metadata is rejected fail-closed.',
                ]);
            }

            if ($payload['policy_target_path'] !== $policy->policy_target_path
                || $payload['policy_target_sha256'] !== $policy->policy_document_sha256
                || $payload['policy_target_length'] !== $policy->policy_document_length) {
                throw ValidationException::withMessages([
                    'policy_target' => 'The authenticated policy target projection does not exactly match the approved Platform policy document.',
                ]);
            }

            $this->assertExactTargets($policy, $targets);

            $digestReplay = ClientUpdateGeneration::query()
                ->where('metadata_set_sha256', $payload['metadata_set_sha256'])
                ->lockForUpdate()
                ->first();
            if ($digestReplay instanceof ClientUpdateGeneration) {
                throw ValidationException::withMessages([
                    'metadata_set_sha256' => 'This public metadata-set identity was already reconciled under another generation identity.',
                ]);
            }

            $latestGeneration = ClientUpdateGeneration::query()
                ->where('channel', $payload['channel'])
                ->orderByDesc('timestamp_version')
                ->lockForUpdate()
                ->first();
            if ($latestGeneration instanceof ClientUpdateGeneration) {
                $versionsMoveForward = $payload['root_version'] >= $latestGeneration->root_version
                    && $payload['targets_version'] >= $latestGeneration->targets_version
                    && $payload['snapshot_version'] >= $latestGeneration->snapshot_version
                    && $payload['timestamp_version'] > $latestGeneration->timestamp_version;

                if (! $versionsMoveForward) {
                    throw ValidationException::withMessages([
                        'metadata_versions' => 'Public TUF metadata versions must not roll back, and Timestamp must advance; stale or replayed generations fail closed.',
                    ]);
                }
            }

            $generation = ClientUpdateGeneration::query()->create([
                'generation_id' => $payload['generation_id'],
                'policy_id' => $policy->id,
                'channel' => $payload['channel'],
                'root_version' => $payload['root_version'],
                'targets_version' => $payload['targets_version'],
                'snapshot_version' => $payload['snapshot_version'],
                'timestamp_version' => $payload['timestamp_version'],
                'metadata_expires_at' => $expiresAt,
                'metadata_set_sha256' => $payload['metadata_set_sha256'],
                'policy_target_path' => $payload['policy_target_path'],
                'policy_target_sha256' => $payload['policy_target_sha256'],
                'policy_target_length' => $payload['policy_target_length'],
                'targets' => $targets,
                'reconciled_at' => now(),
            ]);

            $this->audit->record(
                $actor->id,
                'downloads.updater_generation_reconciled',
                'client_update_generation',
                (string) $generation->id,
                [
                    'generation_id' => $generation->generation_id,
                    'channel' => $generation->channel,
                    'policy_revision' => $policy->revision,
                    'root_version' => $generation->root_version,
                    'targets_version' => $generation->targets_version,
                    'snapshot_version' => $generation->snapshot_version,
                    'timestamp_version' => $generation->timestamp_version,
                    'metadata_expires_at' => $generation->metadata_expires_at->toIso8601String(),
                    'target_count' => count($targets),
                    'source_boundary' => 'protected_release_publishing_integration',
                    'platform_tuf_signature_verification' => false,
                ],
            );

            return $generation->load('policy');
        }, 3);
    }

    /**
     * @param array<string, mixed> $payload
     * @return array{
     *   generation_id: string,
     *   channel: string,
     *   policy_revision: int,
     *   root_version: int,
     *   targets_version: int,
     *   snapshot_version: int,
     *   timestamp_version: int,
     *   metadata_expires_at: string,
     *   metadata_set_sha256: string,
     *   policy_target_path: string,
     *   policy_target_sha256: string,
     *   policy_target_length: int,
     *   targets: list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}>
     * }
     */
    private function validatedPayload(array $payload): array
    {
        if (array_diff(array_keys($payload), self::TOP_LEVEL_KEYS) !== []) {
            throw ValidationException::withMessages([
                'generation_payload' => 'Only the bounded public generation projection is accepted. Private keys, signing secrets and unmodelled metadata are rejected.',
            ]);
        }

        foreach (self::TOP_LEVEL_KEYS as $key) {
            if (! array_key_exists($key, $payload)) {
                throw ValidationException::withMessages([
                    'generation_payload' => "Required public generation field {$key} is missing.",
                ]);
            }
        }

        $generationId = $payload['generation_id'];
        $channel = $payload['channel'];
        $metadataExpiresAt = $payload['metadata_expires_at'];
        $metadataSetSha256 = $payload['metadata_set_sha256'];
        $policyTargetPath = $payload['policy_target_path'];
        $policyTargetSha256 = $payload['policy_target_sha256'];
        $targets = $payload['targets'];

        if (! is_string($generationId)
            || preg_match('/\A[0-9A-Za-z][0-9A-Za-z._:-]{0,127}\z/', $generationId) !== 1) {
            throw ValidationException::withMessages(['generation_id' => 'Generation identity is invalid.']);
        }
        if (! is_string($channel) || ! in_array($channel, DownloadCatalog::channels(), true)) {
            throw ValidationException::withMessages(['channel' => 'Updater channel is invalid.']);
        }

        foreach (['policy_revision', 'root_version', 'targets_version', 'snapshot_version', 'timestamp_version', 'policy_target_length'] as $key) {
            if (! is_int($payload[$key]) || $payload[$key] < 1) {
                throw ValidationException::withMessages([$key => "{$key} must be a positive integer."]);
            }
        }

        if (! is_string($metadataExpiresAt)) {
            throw ValidationException::withMessages(['metadata_expires_at' => 'Metadata expiry must be a timestamp string.']);
        }
        try {
            Carbon::parse($metadataExpiresAt);
        } catch (Throwable) {
            throw ValidationException::withMessages(['metadata_expires_at' => 'Metadata expiry timestamp is invalid.']);
        }

        if (! is_string($metadataSetSha256) || preg_match('/\A[a-f0-9]{64}\z/', $metadataSetSha256) !== 1) {
            throw ValidationException::withMessages(['metadata_set_sha256' => 'Metadata-set SHA-256 must be lowercase hexadecimal.']);
        }
        if (! is_string($policyTargetPath) || $policyTargetPath === '' || strlen($policyTargetPath) > 255) {
            throw ValidationException::withMessages(['policy_target_path' => 'Policy target path is invalid.']);
        }
        if (! is_string($policyTargetSha256) || preg_match('/\A[a-f0-9]{64}\z/', $policyTargetSha256) !== 1) {
            throw ValidationException::withMessages(['policy_target_sha256' => 'Policy target SHA-256 must be lowercase hexadecimal.']);
        }
        if (! is_array($targets) || $targets === [] || count($targets) > 12 || ! array_is_list($targets)) {
            throw ValidationException::withMessages(['targets' => 'Public target projection must be a non-empty bounded list.']);
        }

        $normalizedTargets = [];
        $seenVariants = [];
        $seenPaths = [];
        foreach ($targets as $index => $target) {
            if (! is_array($target)
                || array_diff(array_keys($target), self::TARGET_KEYS) !== []
                || array_diff(self::TARGET_KEYS, array_keys($target)) !== []) {
                throw ValidationException::withMessages([
                    'targets' => "Target {$index} must contain only the exact public target fields.",
                ]);
            }

            $platform = $target['platform'];
            $architecture = $target['architecture'];
            $targetPath = $target['target_path'];
            $length = $target['length'];
            $sha256 = $target['sha256'];

            if (! is_string($platform) || ! in_array($platform, DownloadCatalog::platforms(), true)) {
                throw ValidationException::withMessages(['targets' => "Target {$index} platform is invalid."]);
            }
            if (! is_string($architecture) || ! in_array($architecture, DownloadCatalog::architectures(), true)) {
                throw ValidationException::withMessages(['targets' => "Target {$index} architecture is invalid."]);
            }
            if (! is_string($targetPath) || $targetPath === '' || strlen($targetPath) > 512) {
                throw ValidationException::withMessages(['targets' => "Target {$index} path is invalid."]);
            }
            if (! is_int($length) || $length < 1) {
                throw ValidationException::withMessages(['targets' => "Target {$index} length must be a positive integer."]);
            }
            if (! is_string($sha256) || preg_match('/\A[a-f0-9]{64}\z/', $sha256) !== 1) {
                throw ValidationException::withMessages(['targets' => "Target {$index} SHA-256 must be lowercase hexadecimal."]);
            }

            $variant = $platform.'|'.$architecture;
            if (isset($seenVariants[$variant]) || isset($seenPaths[$targetPath])) {
                throw ValidationException::withMessages([
                    'targets' => 'Platform/architecture variants and exact target paths must be unique.',
                ]);
            }
            $seenVariants[$variant] = true;
            $seenPaths[$targetPath] = true;

            $normalizedTargets[] = [
                'platform' => $platform,
                'architecture' => $architecture,
                'target_path' => $targetPath,
                'length' => $length,
                'sha256' => $sha256,
            ];
        }

        return [
            'generation_id' => $generationId,
            'channel' => $channel,
            'policy_revision' => $payload['policy_revision'],
            'root_version' => $payload['root_version'],
            'targets_version' => $payload['targets_version'],
            'snapshot_version' => $payload['snapshot_version'],
            'timestamp_version' => $payload['timestamp_version'],
            'metadata_expires_at' => $metadataExpiresAt,
            'metadata_set_sha256' => $metadataSetSha256,
            'policy_target_path' => $policyTargetPath,
            'policy_target_sha256' => $policyTargetSha256,
            'policy_target_length' => $payload['policy_target_length'],
            'targets' => $normalizedTargets,
        ];
    }

    /**
     * @param list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}> $targets
     * @return list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}>
     */
    private function normalizeTargets(array $targets): array
    {
        usort(
            $targets,
            static fn (array $left, array $right): int => strcmp(
                $left['platform'].'|'.$left['architecture'].'|'.$left['target_path'],
                $right['platform'].'|'.$right['architecture'].'|'.$right['target_path'],
            ),
        );

        return $targets;
    }

    /** @param list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}> $targets */
    private function assertExactTargets(ClientUpdatePolicy $policy, array $targets): void
    {
        /** @var list<array{artifact_id: int, platform: string, architecture: string, target_path: string, size_bytes: int, supplied_sha256: string}> $expectedTargets */
        $expectedTargets = $policy->artifact_targets;

        if (count($expectedTargets) !== count($targets)) {
            throw ValidationException::withMessages([
                'targets' => 'Public generation target count does not exactly match the approved current release.',
            ]);
        }

        $expectedByVariant = [];
        foreach ($expectedTargets as $expected) {
            $expectedByVariant[$expected['platform'].'|'.$expected['architecture']] = $expected;
        }

        foreach ($targets as $target) {
            $variant = $target['platform'].'|'.$target['architecture'];
            $expected = $expectedByVariant[$variant] ?? null;
            if (! is_array($expected)
                || $target['target_path'] !== $expected['target_path']
                || $target['length'] !== $expected['size_bytes']
                || $target['sha256'] !== $expected['supplied_sha256']) {
                throw ValidationException::withMessages([
                    'targets' => 'Public target metadata must exactly match platform, architecture, immutable target path, expected length and supplied release digest.',
                ]);
            }
        }
    }

    /**
     * @param array{
     *   generation_id: string, channel: string, policy_revision: int, root_version: int,
     *   targets_version: int, snapshot_version: int, timestamp_version: int,
     *   metadata_expires_at: string, metadata_set_sha256: string, policy_target_path: string,
     *   policy_target_sha256: string, policy_target_length: int,
     *   targets: list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}>
     * } $payload
     * @param list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}> $targets
     */
    private function assertExistingGeneration(
        ClientUpdateGeneration $existing,
        array $payload,
        Carbon $expiresAt,
        array $targets,
    ): void {
        $same = $existing->channel === $payload['channel']
            && $existing->policy->revision === $payload['policy_revision']
            && $existing->root_version === $payload['root_version']
            && $existing->targets_version === $payload['targets_version']
            && $existing->snapshot_version === $payload['snapshot_version']
            && $existing->timestamp_version === $payload['timestamp_version']
            && $existing->metadata_expires_at->equalTo($expiresAt)
            && $existing->metadata_set_sha256 === $payload['metadata_set_sha256']
            && $existing->policy_target_path === $payload['policy_target_path']
            && $existing->policy_target_sha256 === $payload['policy_target_sha256']
            && $existing->policy_target_length === $payload['policy_target_length']
            && $this->normalizeTargets($existing->targets) === $targets;

        if (! $same) {
            throw ValidationException::withMessages([
                'generation_id' => 'This generation identity already belongs to different public metadata.',
            ]);
        }
    }
}