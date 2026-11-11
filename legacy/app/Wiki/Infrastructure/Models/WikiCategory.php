<?php

namespace App\Wiki\Infrastructure\Models;

use App\Wiki\Domain\WikiContentRules;
use DomainException;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int|null $parent_id
 * @property string $key
 * @property int $sort_order
 * @property bool $visible
 * @property int $lock_version
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class WikiCategory extends Model
{
    /**
     * @var list<string>
     */
    protected $fillable = [
        'parent_id',
        'key',
        'sort_order',
        'visible',
        'lock_version',
    ];

    protected static function booted(): void
    {
        self::saving(function (self $category): void {
            WikiContentRules::assertCategoryKey($category->key);

            if ($category->exists && $category->parent_id === $category->id) {
                throw new DomainException('A Wiki category cannot be its own parent.');
            }
        });
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'sort_order' => 'integer',
            'visible' => 'boolean',
            'lock_version' => 'integer',
        ];
    }
}
