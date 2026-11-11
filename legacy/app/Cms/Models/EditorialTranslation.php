<?php

namespace App\Cms\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $content_type
 * @property int $content_id
 * @property string $locale
 * @property string|null $title
 * @property string|null $body
 * @property string|null $action_label
 * @property Carbon $source_updated_at
 * @property Carbon|null $published_at
 */
final class EditorialTranslation extends Model
{
    /** @var list<string> */
    protected $fillable = [
        'content_type',
        'content_id',
        'locale',
        'title',
        'body',
        'action_label',
        'source_updated_at',
        'published_at',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'content_id' => 'integer',
            'source_updated_at' => 'datetime',
            'published_at' => 'datetime',
        ];
    }
}
