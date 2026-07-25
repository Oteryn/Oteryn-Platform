<?php

namespace App\Wiki\Infrastructure\Models;

use App\Wiki\Domain\WikiContentRules;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $article_id
 * @property string $locale
 * @property string $title
 * @property string $slug
 * @property string $summary
 * @property string $source_markdown
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class WikiArticleTranslation extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'article_id',
        'locale',
        'title',
        'slug',
        'summary',
        'source_markdown',
    ];

    protected static function booted(): void
    {
        self::saving(function (self $translation): void {
            WikiContentRules::assertArticleTranslation(
                $translation->locale,
                $translation->title,
                $translation->slug,
                $translation->summary,
                $translation->source_markdown,
            );
        });
    }
}
