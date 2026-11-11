<?php

namespace App\Wiki\Infrastructure\Models;

use App\Wiki\Domain\WikiContentRules;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $category_id
 * @property string $locale
 * @property string $name
 * @property string $slug
 * @property string|null $description
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class WikiCategoryTranslation extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'category_id',
        'locale',
        'name',
        'slug',
        'description',
    ];

    protected static function booted(): void
    {
        self::saving(function (self $translation): void {
            WikiContentRules::assertCategoryTranslation(
                $translation->locale,
                $translation->name,
                $translation->slug,
                $translation->description,
            );
        });
    }
}
