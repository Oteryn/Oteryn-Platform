<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
use App\Identity\Models\Identity;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final readonly class ImportSignedUpdaterGeneration
{
    public function __construct(private AdminAuditRecorder $audit) {}

    /**
     * Reconciles a public projection of an already signed TUF repository generation.
     * It never accepts or verifies private signing keys and does not replace client-side TUF signature verification.
     *
     * @param array{
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
     * } $payload
     */
    public function execute(Identity $actor, array $payload): ClientUpdateGeneration
    {
        return DB::transaction(function () use ($actor, $payload): ClientUpdateGeneration {
            $expiresAt = Carbon::parse($payload['metadata_expires_at'])->utc();
            $targets = $this->normalizeTargets($payload['targets']);

            $existing = ClientUpdateGeneration::query()
                ->where('generation_id', $payload['generation_id'])
                ->first();
            if ($existing instanceof ClientUpdateGeneration) {
                $this->assertExistingGeneration($existing, $payload, $expiresAt, $targets);

                return $existing->load('policy');
            }

            $policy = ClientUpdatePolicy::query()
                ->where('channel', $payload['channel'])
                ->where('revision', $payload['policy_revision'])
                ->lockForUpdate()
                ->first();
            if (! $policy instanceof ClientUpdatePolicy) {
                throw ValidationException::withMessages([
                    'channel' => 'The signed generation does not match an approved policy revision in this channel.',
                ]);
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
                    'metadata_expires_at' => 'Expired signed-generation metadata is rejected fail-closed.',
                ]);
            }

            if ($payload['policy_target_path'] !== $policy->policy_target_path
                || $payload['policy_target_sha256'] !== $policy->policy_document_sha256
                || $payload['policy_target_length'] !== $policy->policy_document_length) {
                throw ValidationException::withMessages([
                    'policy_target' => 'The signed policy target does not exactly match the approved Platform policy document.',
                ]);
            }

            $this->assertExactTargets($policy, $targets);

            $digestReplay = ClientUpdateGeneration::query()
                ->where('metadata_set_sha256', $payload['metadata_set_sha256'])
                ->lockForUpdate()
                ->first();
            if ($digestReplay instanceof ClientUpdateGeneration) {
                throw ValidationException::withMessages([
                    'metadata_set_sha256' => 'This signed metadata-set identity was already reconciled under another generation identity.',
                ]);
            }

            $latestGeneration = ClientUpdateGeneration::query()
                ->where('channel', $payload['channel'])
                ->orderByDesc('timestamp_version')
                ->lockForUpdate()
                ->first();
            if ($latestGeneration instanceof ClientUpdateGeneration) {
                $versionsMoveForward = $payload['root_version'] >= $latestGeneration->root_version
                    && $payload['targets_version'] > $latestGeneration->targets_version
                    && $payload['snapshot_version'] > $latestGeneration->snapshot_version
                    && $payload['timestamp_version'] > $latestGeneration->timestamp_version;

                if (! $versionsMoveForward) {
                    throw ValidationException::withMessages([
                        'metadata_versions' => 'Signed metadata versions must move forward; stale or replayed generations fail closed.',
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
                ],
            );

            return $generation->load('policy');
        }, 3);
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
                'targets' => 'Signed generation target count does not exactly match the approved current release.',
            ]);
        }

        $expectedByVariant = [];
        foreach ($expectedTargets as $expected) {
            $expectedByVariant[$expected['platform'].'|'.$expected['architecture']] = $expected;
        }

        $seenVariants = [];
        foreach ($targets as $target) {
            $variant = $target['platform'].'|'.$target['architecture'];
            if (isset($seenVariants[$variant])) {
                throw ValidationException::withMessages([
                    'targets' => 'A signed generation cannot contain duplicate platform/architecture targets.',
                ]);
            }
            $seenVariants[$variant] = true;

            $expected = $expectedByVariant[$variant] ?? null;
            if (! is_array($expected)
                || $target['target_path'] !== $expected['target_path']
                || $target['length'] !== $expected['size_bytes']
                || $target['sha256'] !== $expected['supplied_sha256']) {
                throw ValidationException::withMessages([
                    'targets' => 'Signed target metadata must exactly match platform, architecture, immutable target path, expected length and supplied release digest.',
                ]);
            }
        }
    }

    /**
     * @param array{generation_id: string, channel: string, policy_revision: int, root_version: int, targets_version: int, snapshot_version: int, timestamp_version: int, metadata_expires_at: string, metadata_set_sha256: string, policy_target_path: string, policy_target_sha256: string, policy_target_length: int, targets: list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}>} $payload
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
                'generation_id' => 'This generation identity already belongs to different public signed metadata.',
            ]);
        }
    }
}