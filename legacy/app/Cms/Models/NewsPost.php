<?php

namespace App\Cms\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $slug
 * @property string $title
 * @property string $body
 * @property Carbon|null $published_at
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class NewsPost extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'slug',
        'title',
        'body',
        'published_at',
    ];

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'published_at' => 'datetime',
        ];
    }
}
