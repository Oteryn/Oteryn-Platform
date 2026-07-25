<?php

namespace App\Events\Models;

use App\Events\Factories\EventTranslationFactory;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

/**
 * @property int $id
 * @property int $event_id
 * @property string $locale
 * @property string $title
 * @property string $slug
 * @property string $summary
 * @property string $body
 */
final class EventTranslation extends Model
{
    /** @use HasFactory<EventTranslationFactory> */
    use HasFactory;

    /**
     * @var list<string>
     */
    protected $fillable = [
        'event_id',
        'locale',
        'title',
        'slug',
        'summary',
        'body',
    ];

    protected static function newFactory(): EventTranslationFactory
    {
        return EventTranslationFactory::new();
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'event_id' => 'integer',
        ];
    }
}
