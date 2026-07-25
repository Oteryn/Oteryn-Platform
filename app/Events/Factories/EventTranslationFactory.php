<?php

namespace App\Events\Factories;

use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use Illuminate\Database\Eloquent\Factories\Factory;
use Illuminate\Support\Str;

/**
 * @extends Factory<EventTranslation>
 */
final class EventTranslationFactory extends Factory
{
    /** @var class-string<EventTranslation> */
    protected $model = EventTranslation::class;

    /**
     * @return array<string, mixed>
     */
    public function definition(): array
    {
        $token = Str::lower(Str::random(20));

        return [
            'event_id' => Event::factory(),
            'locale' => 'en',
            'title' => 'Event '.$token,
            'slug' => 'event-'.$token,
            'summary' => 'Event summary '.$token.'.',
            'body' => 'Event details '.$token.'.',
        ];
    }
}
