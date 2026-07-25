<?php

namespace App\Wiki\Infrastructure\Models;

use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiContentRules;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $content_type
 * @property WikiArticleStatus $status
 * @property bool $is_featured
 * @property int $sort_order
 * @property int|null $author_identity_id
 * @property int|null $last_editor_identity_id
 * @property int|null $publisher_identity_id
 * @property Carbon|null $published_at
 * @property int $lock_version
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class WikiArticle extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'content_type',
        'status',
        'is_featured',
        'sort_order',
        'author_identity_id',
        'last_editor_identity_id',
        'publisher_identity_id',
        'published_at',
        'lock_version',
    ];

    protected static function booted(): void
    {
        self::saving(function (self $article): void {
            WikiContentRules::assertContentType($article->content_type);
        });
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'status' => WikiArticleStatus::class,
            'is_featured' => 'boolean',
            'sort_order' => 'integer',
            'published_at' => 'datetime',
            'lock_version' => 'integer',
        ];
    }
}
