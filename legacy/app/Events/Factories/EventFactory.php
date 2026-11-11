<?php

namespace App\Events\Factories;

use App\Events\Models\Event;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<Event>
 */
final class EventFactory extends Factory
{
    /** @var class-string<Event> */
    protected $model = Event::class;

    /**
     * @return array<string, mixed>
     */
    public function definition(): array
    {
        return [
            'status' => Event::STATUS_DRAFT,
            'starts_at' => now()->addDay(),
            'ends_at' => now()->addDay()->addHours(2),
            'featured' => false,
            'news_post_id' => null,
            'lock_version' => 1,
        ];
    }
}
