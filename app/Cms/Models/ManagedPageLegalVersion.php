<?php

namespace App\Cms\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $managed_page_id
 * @property string $version
 * @property Carbon $effective_date
 * @property string $title
 * @property string $body
 * @property Carbon $published_at
 */
final class ManagedPageLegalVersion extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'managed_page_id',
        'version',
        'effective_date',
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
            'effective_date' => 'date',
            'published_at' => 'datetime',
        ];
    }
}
