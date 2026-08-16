<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final readonly class ActivateUpdaterGeneration
{
    public function __construct(private AdminAuditRecorder $audit) {}

    public function execute(Identity $actor, ClientUpdateGeneration $generation): ClientUpdateGeneration
    {
        $generationId = $generation->id;
        $channel = $generation->channel;

        return DB::transaction(function () use ($actor, $generationId, $channel): ClientUpdateGeneration {
            $channelGenerations = ClientUpdateGeneration::query()
                ->where('channel', $channel)
                ->with('policy')
                ->orderBy('id')
                ->lockForUpdate()
                ->get();
            $stored = $channelGenerations->first(
                static fn (ClientUpdateGeneration $candidate): bool => $candidate->id === $generationId,
            );

            if (! $stored instanceof ClientUpdateGeneration) {
                throw ValidationException::withMessages([
                    'generation' => 'The updater generation no longer exists in this channel.',
                ]);
            }

            if ($stored->activated_at !== null && $stored->superseded_at === null) {
                return $stored;
            }

            if ($stored->activated_at !== null || $stored->superseded_at !== null) {
                throw ValidationException::withMessages([
                    'generation' => 'A superseded updater generation cannot be reactivated.',
                ]);
            }

            if (! $stored->metadata_expires_at->isFuture()) {
                throw ValidationException::withMessages([
                    'generation' => 'Expired updater metadata cannot become Platform-active.',
                ]);
            }

            $latestPolicy = ClientUpdatePolicy::query()
                ->where('channel', $channel)
                ->orderByDesc('revision')
                ->lockForUpdate()
                ->first();
            if (! $latestPolicy instanceof ClientUpdatePolicy || $latestPolicy->id !== $stored->policy_id) {
                throw ValidationException::withMessages([
                    'generation' => 'Only the latest approved channel policy generation can become Platform-active.',
                ]);
            }

            $latestImported = $channelGenerations->sortByDesc('timestamp_version')->first();
            if (! $latestImported instanceof ClientUpdateGeneration || $latestImported->id !== $stored->id) {
                throw ValidationException::withMessages([
                    'generation' => 'A stale imported generation cannot become Platform-active after a newer generation is reconciled.',
                ]);
            }

            $currentRelease = ClientRelease::query()
                ->lockForUpdate()
                ->find($latestPolicy->current_release_id);
            if (! $currentRelease instanceof ClientRelease
                || $currentRelease->updater_release_id === null
                || $currentRelease->updater_withdrawn_at !== null) {
                throw ValidationException::withMessages([
                    'generation' => 'The policy current release is unavailable or withdrawn; activation fails closed.',
                ]);
            }

            $activatedAt = now();
            ClientUpdateGeneration::query()
                ->where('channel', $channel)
                ->where('id', '!=', $stored->id)
                ->whereNotNull('activated_at')
                ->whereNull('superseded_at')
                ->update([
                    'superseded_at' => $activatedAt,
                    'updated_at' => $activatedAt,
                ]);

            $stored->forceFill([
                'activated_at' => $activatedAt,
                'superseded_at' => null,
            ])->save();

            $this->audit->record(
                $actor->id,
                'downloads.updater_generation_activated',
                'client_update_generation',
                (string) $stored->id,
                [
                    'generation_id' => $stored->generation_id,
                    'channel' => $stored->channel,
                    'policy_revision' => $latestPolicy->revision,
                    'timestamp_version' => $stored->timestamp_version,
                    'production_activation' => false,
                ],
            );

            return $stored->refresh()->load('policy');
        }, 3);
    }
}