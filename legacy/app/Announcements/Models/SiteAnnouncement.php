<?php

namespace App\Announcements\Models;

use App\Announcements\Factories\SiteAnnouncementFactory;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $title
 * @property string $body
 * @property string $severity
 * @property Carbon $starts_at
 * @property Carbon|null $ends_at
 * @property string $publication_state
 * @property string|null $action_label
 * @property string|null $action_url
 * @property int|null $created_by
 * @property int|null $updated_by
 * @property int|null $published_by
 * @property int $lock_version
 */
final class SiteAnnouncement extends Model
{
    /** @use HasFactory<SiteAnnouncementFactory> */
    use HasFactory;

    public const SEVERITY_INFO = 'info';

    public const SEVERITY_MAINTENANCE = 'maintenance';

    public const SEVERITY_WARNING = 'warning';

    public const STATE_DRAFT = 'draft';

    public const STATE_PUBLISHED = 'published';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'title',
        'body',
        'severity',
        'starts_at',
        'ends_at',
        'publication_state',
        'action_label',
        'action_url',
        'created_by',
        'updated_by',
        'published_by',
        'lock_version',
    ];

    /**
     * @return list<string>
     */
    public static function severities(): array
    {
        return [
            self::SEVERITY_INFO,
            self::SEVERITY_MAINTENANCE,
            self::SEVERITY_WARNING,
        ];
    }

    /**
     * @return list<string>
     */
    public static function publicationStates(): array
    {
        return [
            self::STATE_DRAFT,
            self::STATE_PUBLISHED,
        ];
    }

    protected static function newFactory(): SiteAnnouncementFactory
    {
        return SiteAnnouncementFactory::new();
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'starts_at' => 'datetime',
            'ends_at' => 'datetime',
            'created_by' => 'integer',
            'updated_by' => 'integer',
            'published_by' => 'integer',
            'lock_version' => 'integer',
        ];
    }
}
