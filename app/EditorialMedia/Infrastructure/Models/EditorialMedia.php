<?php

namespace App\EditorialMedia\Infrastructure\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $disk
 * @property string $storage_path
 * @property string|null $thumbnail_path
 * @property string|null $original_name
 * @property string $mime_type
 * @property string $extension
 * @property int $byte_size
 * @property int $width
 * @property int $height
 * @property int|null $thumbnail_byte_size
 * @property int|null $thumbnail_width
 * @property int|null $thumbnail_height
 * @property string $sha256
 * @property string $alt_text
 * @property int|null $uploaded_by_identity_id
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class EditorialMedia extends Model
{
    protected $table = 'editorial_media';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'disk',
        'storage_path',
        'thumbnail_path',
        'original_name',
        'mime_type',
        'extension',
        'byte_size',
        'width',
        'height',
        'thumbnail_byte_size',
        'thumbnail_width',
        'thumbnail_height',
        'sha256',
        'alt_text',
        'uploaded_by_identity_id',
    ];

    /**
     * @return HasMany<EditorialMediaReference, $this>
     */
    public function references(): HasMany
    {
        return $this->hasMany(EditorialMediaReference::class, 'media_id');
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'byte_size' => 'integer',
            'width' => 'integer',
            'height' => 'integer',
            'thumbnail_byte_size' => 'integer',
            'thumbnail_width' => 'integer',
            'thumbnail_height' => 'integer',
            'uploaded_by_identity_id' => 'integer',
        ];
    }
}
