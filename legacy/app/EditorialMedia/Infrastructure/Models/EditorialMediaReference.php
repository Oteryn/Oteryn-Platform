<?php

namespace App\EditorialMedia\Infrastructure\Models;

use App\EditorialMedia\Domain\EditorialMediaConsumer;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $media_id
 * @property EditorialMediaConsumer $consumer
 * @property string $consumer_id
 * @property string $usage
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class EditorialMediaReference extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'media_id',
        'consumer',
        'consumer_id',
        'usage',
    ];

    /**
     * @return BelongsTo<EditorialMedia, $this>
     */
    public function media(): BelongsTo
    {
        return $this->belongsTo(EditorialMedia::class, 'media_id');
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'media_id' => 'integer',
            'consumer' => EditorialMediaConsumer::class,
        ];
    }
}
