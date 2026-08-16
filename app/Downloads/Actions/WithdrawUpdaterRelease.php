<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\Models\ClientRelease;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final readonly class WithdrawUpdaterRelease
{
    public function __construct(private AdminAuditRecorder $audit) {}

    public function execute(Identity $actor, ClientRelease $release): ClientRelease
    {
        return DB::transaction(function () use ($actor, $release): ClientRelease {
            $storedRelease = ClientRelease::query()->lockForUpdate()->find($release->id);

            if (! $storedRelease instanceof ClientRelease || $storedRelease->updater_release_id === null) {
                throw ValidationException::withMessages([
                    'release' => 'The release is not updater-enabled.',
                ]);
            }

            if ($storedRelease->updater_withdrawn_at !== null) {
                return $storedRelease->load('artifacts');
            }

            $storedRelease->forceFill(['updater_withdrawn_at' => now()])->save();

            $this->audit->record(
                $actor->id,
                'downloads.updater_release_withdrawn',
                'client_release',
                (string) $storedRelease->id,
                [
                    'channel' => $storedRelease->channel,
                    'updater_release_id' => $storedRelease->updater_release_id,
                    'updater_sequence' => $storedRelease->updater_sequence,
                ],
            );

            return $storedRelease->load('artifacts');
        }, 3);
    }
}
