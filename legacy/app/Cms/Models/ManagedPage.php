<?php

namespace App\Cms\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $slug
 * @property string $title
 * @property string $body
 * @property string|null $legal_version
 * @property Carbon|null $legal_effective_date
 * @property Carbon|null $published_at
 */
final class ManagedPage extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'slug',
        'title',
        'body',
        'legal_version',
        'legal_effective_date',
        'published_at',
    ];

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'legal_effective_date' => 'date',
            'published_at' => 'datetime',
        ];
    }
}
