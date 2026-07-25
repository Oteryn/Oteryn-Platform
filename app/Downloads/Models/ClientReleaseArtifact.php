<?php

namespace App\Downloads\Models;

use App\Downloads\DownloadCatalog;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

/**
 * @property int $id
 * @property int $client_release_id
 * @property string $platform
 * @property string $architecture
 * @property string $artifact_url
 * @property string $filename
 * @property int $size_bytes
 * @property string $sha256
 * @property bool $is_enabled
 * @property-read ClientRelease $release
 */
final class ClientReleaseArtifact extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'platform',
        'architecture',
        'artifact_url',
        'filename',
        'size_bytes',
        'sha256',
        'is_enabled',
    ];

    /**
     * @return BelongsTo<ClientRelease, $this>
     */
    public function release(): BelongsTo
    {
        return $this->belongsTo(ClientRelease::class, 'client_release_id');
    }

    public function formattedSize(): string
    {
        return DownloadCatalog::formatBytes($this->size_bytes);
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'size_bytes' => 'integer',
            'is_enabled' => 'boolean',
        ];
    }
}
