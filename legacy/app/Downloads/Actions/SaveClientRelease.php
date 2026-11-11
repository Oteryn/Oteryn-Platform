<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\Models\ClientRelease;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final readonly class SaveClientRelease
{
    public function __construct(private AdminAuditRecorder $audit) {}

    /**
     * @param  list<array{platform: string, architecture: string, artifact_url: string, filename: string, size_bytes: int, sha256: string, is_enabled: bool}>  $artifacts
     */
    public function execute(
        Identity $actor,
        ?ClientRelease $release,
        string $version,
        string $channel,
        ?string $releaseNotes,
        array $artifacts,
    ): ClientRelease {
        $releaseId = $release?->id;

        return DB::transaction(function () use (
            $actor,
            $releaseId,
            $version,
            $channel,
            $releaseNotes,
            $artifacts,
        ): ClientRelease {
            $created = $releaseId === null;
            $storedRelease = $created
                ? new ClientRelease
                : ClientRelease::query()->lockForUpdate()->findOrFail($releaseId);

            if ($storedRelease->published_at !== null) {
                throw ValidationException::withMessages([
                    'release' => 'Published releases are immutable. Create a new release for changed artifact metadata.',
                ]);
            }

            $storedRelease->fill([
                'version' => $version,
                'channel' => $channel,
                'release_notes' => $releaseNotes,
                'published_at' => null,
                'is_current' => false,
            ]);
            $storedRelease->save();

            $storedRelease->artifacts()->delete();
            $storedRelease->artifacts()->createMany($artifacts);

            $enabledArtifactCount = 0;

            foreach ($artifacts as $artifact) {
                if ($artifact['is_enabled']) {
                    $enabledArtifactCount++;
                }
            }

            $this->audit->record(
                $actor->id,
                $created ? 'downloads.release_created' : 'downloads.release_updated',
                'client_release',
                (string) $storedRelease->id,
                [
                    'version' => $storedRelease->version,
                    'channel' => $storedRelease->channel,
                    'artifact_count' => count($artifacts),
                    'enabled_artifact_count' => $enabledArtifactCount,
                ],
            );

            return $storedRelease->load('artifacts');
        }, 3);
    }
}
