<?php

namespace App\Announcements\Factories;

use App\Announcements\Models\SiteAnnouncement;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<SiteAnnouncement>
 */
final class SiteAnnouncementFactory extends Factory
{
    /** @var class-string<SiteAnnouncement> */
    protected $model = SiteAnnouncement::class;

    /**
     * @return array<string, mixed>
     */
    public function definition(): array
    {
        return [
            'title' => 'Scheduled announcement',
            'body' => 'Scheduled announcement details.',
            'severity' => SiteAnnouncement::SEVERITY_INFO,
            'starts_at' => now()->subHour(),
            'ends_at' => now()->addDay(),
            'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
            'action_label' => null,
            'action_url' => null,
            'lock_version' => 1,
        ];
    }
}
