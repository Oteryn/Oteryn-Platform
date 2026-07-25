<?php

namespace App\Events\Models;

use App\Events\Factories\EventFactory;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $status
 * @property Carbon $starts_at
 * @property Carbon $ends_at
 * @property bool $featured
 * @property int|null $news_post_id
 * @property int|null $created_by
 * @property int|null $updated_by
 * @property int|null $published_by
 * @property int $lock_version
 */
final class Event extends Model
{
    /** @use HasFactory<EventFactory> */
    use HasFactory;

    public const STATUS_DRAFT = 'draft';

    public const STATUS_SCHEDULED = 'scheduled';

    public const STATUS_ACTIVE = 'active';

    public const STATUS_COMPLETED = 'completed';

    public const STATUS_CANCELLED = 'cancelled';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'status',
        'starts_at',
        'ends_at',
        'featured',
        'news_post_id',
        'created_by',
        'updated_by',
        'published_by',
        'lock_version',
    ];

    /**
     * @return list<string>
     */
    public static function statuses(): array
    {
        return [
            self::STATUS_DRAFT,
            self::STATUS_SCHEDULED,
            self::STATUS_ACTIVE,
            self::STATUS_COMPLETED,
            self::STATUS_CANCELLED,
        ];
    }

    /**
     * @return list<string>
     */
    public static function publicStatuses(): array
    {
        return [
            self::STATUS_SCHEDULED,
            self::STATUS_ACTIVE,
            self::STATUS_COMPLETED,
            self::STATUS_CANCELLED,
        ];
    }

    public function effectiveStatusAt(DateTimeInterface $readTime): string
    {
        if ($this->status === self::STATUS_DRAFT || $this->status === self::STATUS_CANCELLED) {
            return $this->status;
        }

        $at = CarbonImmutable::instance($readTime)->utc();
        $startsAt = CarbonImmutable::instance($this->starts_at)->utc();
        $endsAt = CarbonImmutable::instance($this->ends_at)->utc();

        if ($at->lt($startsAt)) {
            return self::STATUS_SCHEDULED;
        }

        if ($at->greaterThanOrEqualTo($endsAt)) {
            return self::STATUS_COMPLETED;
        }

        return self::STATUS_ACTIVE;
    }

    protected static function newFactory(): EventFactory
    {
        return EventFactory::new();
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'starts_at' => 'datetime',
            'ends_at' => 'datetime',
            'featured' => 'boolean',
            'news_post_id' => 'integer',
            'created_by' => 'integer',
            'updated_by' => 'integer',
            'published_by' => 'integer',
            'lock_version' => 'integer',
        ];
    }
}
