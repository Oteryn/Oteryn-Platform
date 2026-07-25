<?php

namespace App\Downloads\Models;

use Illuminate\Database\Eloquent\Collection;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $version
 * @property string $channel
 * @property string|null $release_notes
 * @property Carbon|null $published_at
 * @property bool $is_current
 * @property-read Collection<int, ClientReleaseArtifact> $artifacts
 */
final class ClientRelease extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'version',
        'channel',
        'release_notes',
        'published_at',
        'is_current',
    ];

    /**
     * @return HasMany<ClientReleaseArtifact, $this>
     */
    public function artifacts(): HasMany
    {
        return $this->hasMany(ClientReleaseArtifact::class);
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'published_at' => 'datetime',
            'is_current' => 'boolean',
        ];
    }
}
